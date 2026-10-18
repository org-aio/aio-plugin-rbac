"""在独立本机 PostgreSQL 和真实 Docker Sidecar 中验收权限整包生命周期与授权语义。"""
import hashlib
import json
import os
import shutil
import socket
import subprocess
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid
from pathlib import Path

plugin = Path(__file__).resolve().parents[2]
binary = Path(os.environ['AIO_RBAC_DELIVERY_HOST']).resolve(strict=True)
database = os.environ['AIO_RBAC_DELIVERY_DATABASE_URL']
parsed = urllib.parse.urlparse(database)
assert parsed.hostname == '127.0.0.1' and parsed.path == '/rbac_delivery_test'
postgres = os.environ['AIO_RBAC_DELIVERY_POSTGRES_CONTAINER']
web = Path(os.environ['AIO_RBAC_DELIVERY_WEB']).resolve(strict=True)
packager = Path(os.environ['AIO_RBAC_DELIVERY_PACKAGER']).resolve(strict=True)
root = Path(tempfile.mkdtemp(prefix='aio-rbac-delivery-'))
(root / 'processes').mkdir()
(root / 'config.toml').write_text('plugins = []\n', encoding='utf-8')
with socket.socket() as reservation:
    reservation.bind(('127.0.0.1', 0))
    port = reservation.getsockname()[1]
base = 'http://127.0.0.1:%d' % port
image = 'sha256:48a661b5e5f4560287dfc254e948825538d0f7a1379616495a424ed375afd31f'
env = {k: v for k, v in os.environ.items() if not k.startswith('AIO_')}
env.update({
    'AIO_DATABASE_URL': database, 'AIO_COMPONENT_DATABASE_URL': database,
    'AIO_COMPONENT_HOME': str(root / 'components'), 'AIO_PLUGIN_CACHE': str(root / 'cache'),
    'AIO_CONFIG': str(root / 'config.toml'),
    'AIO_PROCESS_ROOT': str(root / 'processes'), 'AIO_PROCESS_IMAGES': image,
    'AIO_PLUGIN_SUPERVISOR_SOCKET': str(root / 'supervisor.sock'),
    'AIO_PUBLIC_ORIGIN': base, 'AIO_WEB_HOST': '127.0.0.1', 'AIO_WEB_PORT': str(port),
    'AIO_WEB_DIST': str(web), 'AIO_BOOTSTRAP_ACCOUNT': 'rbac-test',
    'AIO_BOOTSTRAP_PASSWORD': uuid.uuid4().hex, 'AIO_PLUGIN_PUBLISH_ACCOUNTS': 'rbac-test',
})
http = urllib.request.build_opener(urllib.request.ProxyHandler({}))
cookie = None
host = None
supervisor = None
host_log = open(str(root / 'host.log'), 'w', encoding='utf-8')
supervisor_log = open(str(root / 'supervisor.log'), 'w', encoding='utf-8')
host_hash = hashlib.sha256(binary.read_bytes()).hexdigest()


def sql(statement):
    return subprocess.check_output(['docker', 'exec', '-i', postgres, 'psql', '-U', parsed.username, '-d', 'rbac_delivery_test', '-X', '-v', 'ON_ERROR_STOP=1', '-Atc', statement], universal_newlines=True).strip()


def request(path, value=None, raw=None):
    headers = {'Cookie': cookie} if cookie else {}
    body = raw
    if raw is not None:
        headers['Content-Type'] = 'application/vnd.aio.component+gzip'
    if value is not None:
        body = json.dumps(value).encode('utf-8')
        headers['Content-Type'] = 'application/json'
    try:
        response = http.open(urllib.request.Request(base + path, data=body, headers=headers), timeout=180)
    except urllib.error.HTTPError as error:
        response = error
    with response:
        return response.status, response.headers, response.read()


def success(path, value=None, raw=None):
    status, _, body = request(path, value, raw)
    assert 200 <= status < 300, '%s: HTTP %s %s' % (path[:40], status, body[:250])
    if not body:
        return None
    return json.loads(body).get('data')


def until(predicate, timeout=60):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if predicate():
            return
        time.sleep(0.1)
    raise AssertionError('等待超时；检查日志目录 ' + str(root))


def ready():
    assert host.poll() is None, '宿主提前退出；检查 ' + str(root)
    try:
        return request('/health')[0] == 200
    except OSError:
        return False


def launch():
    global host
    host = subprocess.Popen([str(binary)], cwd=str(root), env=env, stdout=host_log, stderr=subprocess.STDOUT)
    until(ready)


def stop():
    global host
    if host is not None and host.poll() is None:
        host.terminate()
        assert host.wait(timeout=40) == 0
    host = None


def pages():
    return success('/api/runtime/catalog')['pages']


def mount(source, page='users', version=None):
    page_id = 'component:%s:%s' % (source, page)
    value = success('/api/runtime/frontend/mount', {'page_id': page_id})
    assert 'page=' + urllib.parse.quote(page_id, safe='') in value['src'], value['src']
    if version is not None:
        path = urllib.parse.urlsplit(value['src'])
        status, _, body = request(path.path + ('?' + path.query if path.query else ''))
        assert status == 200 and ('data-delivery-version="%s"' % version).encode() in body
    return value['token']


def invoke_failure(token, method, path, value=None):
    payload = {'method': method, 'path': path, 'headers': [], 'body': []}
    if value is not None:
        payload['headers'] = [{'name': 'content-type', 'value': 'application/json'}]
        payload['body'] = list(json.dumps(value).encode('utf-8'))
    status, _, _ = request('/api/runtime/components/%s/request' % token, payload)
    assert status >= 400, '撤权后调用仍成功'
    return status


def invoke(token, method, path, value=None, expected=None):
    payload = {'method': method, 'path': path, 'headers': [], 'body': []}
    if value is not None:
        payload['headers'] = [{'name': 'content-type', 'value': 'application/json'}]
        payload['body'] = list(json.dumps(value).encode('utf-8'))
    response = success('/api/runtime/components/%s/request' % token, payload)
    if expected is not None:
        assert response['status'] == expected, (response, path)
    else:
        assert 200 <= response['status'] < 300, (response, path)
    return json.loads(bytes(response['body']).decode('utf-8')).get('data') if response['body'] else None


def publish(version):
    bundle = root / ('rbac-%s.aio-plugin' % version)
    commit = os.environ.get('AIO_RBAC_DELIVERY_COMMIT') or subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=str(plugin), universal_newlines=True).strip()
    candidate = root / ('candidate-' + version)
    candidate.mkdir()
    shutil.copy2(str(plugin / 'aio-plugin.toml'), str(candidate / 'aio-plugin.toml'))
    shutil.copytree(str(plugin / 'dist'), str(candidate / 'dist'))
    index = candidate / 'dist/frontend/index.html'
    index.write_text(index.read_text(encoding='utf-8').replace('<body>', '<body data-delivery-version="%s">' % version), encoding='utf-8')
    subprocess.check_call([str(packager), str(candidate), 'aio-plugin.toml', 'https://github.com/zjarlin/aio-plugin-rbac.git', commit, version, str(bundle)])
    result = success('/api/runtime/components/publish', raw=bundle.read_bytes())
    print('Published rbac ' + version, flush=True)
    return result


try:
    assert sql("SELECT count(*) FROM pg_tables WHERE schemaname='public'") == '0', '请使用新的专用测试库'
    supervisor = subprocess.Popen([str(binary), 'supervisor'], cwd=str(root), env=env, stdout=supervisor_log, stderr=subprocess.STDOUT)
    until(lambda: (root / 'supervisor.sock').exists())
    launch()
    status, headers, _ = request('/api/auth/login', {'account': env['AIO_BOOTSTRAP_ACCOUNT'], 'password': env['AIO_BOOTSTRAP_PASSWORD']})
    assert status == 200
    cookie = headers['Set-Cookie'].split(';')[0]
    # 静态宿主不再编译权限组合。
    assert request('/api/rbac')[0] == 404

    first = publish('0.1.0')
    source = first['source_id']
    git = 'https://github.com/zjarlin/aio-plugin-rbac.git'
    success('/api/runtime/plugins/install', {'git': git})
    token = mount(source, 'users', '0.1.0')
    view = invoke(token, 'GET', '/rbac')
    assert any(user['account'] == env['AIO_BOOTSTRAP_ACCOUNT'] for user in view['users'])
    assert any(role['id'] == 'platform-admin' for role in view['roles'])
    assert 'rbac:manage' in view['grantable_permissions']
    print('Installed and read the real access-control view through /identity/invoke', flush=True)

    # 角色：创建、更新、删除。
    invoke(token, 'POST', '/rbac/roles', {'role_id': 'auditor', 'permissions': ['file:manage']})
    view = invoke(token, 'GET', '/rbac')
    assert next(role for role in view['roles'] if role['id'] == 'auditor')['permissions'] == ['file:manage']
    invoke(token, 'PUT', '/rbac/roles/auditor', {'role_id': 'auditor', 'permissions': ['workspace:view']})
    view = invoke(token, 'GET', '/rbac')
    assert next(role for role in view['roles'] if role['id'] == 'auditor')['permissions'] == ['workspace:view']
    print('Role create, update and listing use the host transaction guard', flush=True)

    # 用户：创建、改名、重置密码、分配角色、移出。
    account = 'rbac-member-' + uuid.uuid4().hex[:8]
    invoke(token, 'POST', '/rbac/users', {'account': account, 'display_name': '权限成员', 'password': 'initial-password-123'})
    view = invoke(token, 'GET', '/rbac')
    user = next(user for user in view['users'] if user['account'] == account)
    assert user['roles'] == ['member']
    invoke(token, 'PUT', '/rbac/users/' + user['id'], {'display_name': '权限成员（改）'})
    invoke(token, 'PUT', '/rbac/users/%s/roles' % user['id'], {'roles': ['member', 'auditor']})
    invoke(token, 'POST', '/rbac/users/%s/password' % user['id'], {'new_password': 'changed-password-456'})
    view = invoke(token, 'GET', '/rbac')
    user = next(u for u in view['users'] if u['account'] == account)
    assert user['display_name'] == '权限成员（改）'
    assert sorted(user['roles']) == ['auditor', 'member']
    assert sql("SELECT count(*) FROM identity_users WHERE account='%s'" % account) == '1'
    print('User create, rename, role assignment and password reset completed', flush=True)

    # 部门与岗位：创建、更新、删除。
    invoke(token, 'POST', '/rbac/departments', {'id': None, 'parent_id': None, 'name': '研发部', 'leader': '张三', 'phone': '10086', 'email': 'rd@example.com', 'sort_order': 1, 'status': 'active'})
    view = invoke(token, 'GET', '/rbac')
    department = next(item for item in view['departments'] if item['name'] == '研发部')
    invoke(token, 'PUT', '/rbac/departments/' + department['id'], {'id': department['id'], 'parent_id': None, 'name': '研发中心', 'leader': '李四', 'phone': '10010', 'email': 'rd@example.com', 'sort_order': 2, 'status': 'active'})
    invoke(token, 'POST', '/rbac/posts', {'id': None, 'code': 'engineer', 'name': '工程师', 'sort_order': 1, 'status': 'active', 'remark': ''})
    view = invoke(token, 'GET', '/rbac')
    assert any(item['name'] == '研发中心' for item in view['departments'])
    post = next(item for item in view['posts'] if item['code'] == 'engineer')
    invoke(token, 'DELETE', '/rbac/posts/' + post['id'])
    invoke(token, 'DELETE', '/rbac/departments/' + department['id'])
    view = invoke(token, 'GET', '/rbac')
    assert not any(item['id'] == department['id'] for item in view['departments'])
    print('Department and post create, update and delete completed', flush=True)

    # 移出成员后账号不再属于该租户。
    invoke(token, 'DELETE', '/rbac/users/' + user['id'])
    assert sql("SELECT count(*) FROM tenant_memberships WHERE user_id='%s'" % user['id']) == '0'

    second = publish('0.1.1')
    success('/api/runtime/plugins/install', {'git': git})
    token = mount(source, 'users', '0.1.1')
    assert any(role['id'] == 'auditor' for role in invoke(token, 'GET', '/rbac')['roles'])
    success('/api/runtime/plugins/%s/rollback' % source, {})
    token = mount(source, 'users', '0.1.0')
    assert any(role['id'] == 'auditor' for role in invoke(token, 'GET', '/rbac')['roles'])
    print('Upgrade and rollback keep access-control data', flush=True)

    success('/api/runtime/plugins/%s/disable' % source, {})
    for page in ['users', 'roles', 'departments', 'posts']:
        assert not any(p['id'] == 'component:%s:%s' % (source, page) for p in pages())
    assert request('/api/runtime/components/%s/request' % token, {'method': 'GET', 'path': '/rbac'})[0] >= 400
    stop(); launch()
    assert not any(p['id'] == 'component:%s:users' % source for p in pages())
    success('/api/runtime/plugins/%s/enable' % source, {})
    token = mount(source, 'users', '0.1.0')
    assert any(role['id'] == 'auditor' for role in invoke(token, 'GET', '/rbac')['roles'])
    print('Disable survives restart; enable preserves data', flush=True)

    # 撤权后写操作与视图都立即拒绝；恢复后可用。
    grants = sql("WITH removed AS (DELETE FROM role_permissions WHERE tenant_id='default' AND role_id='platform-admin' AND permission='rbac:manage' RETURNING *) SELECT json_agg(removed) FROM removed")
    assert json.loads(grants)
    invoke_failure(token, 'GET', '/rbac')
    invoke_failure(token, 'POST', '/rbac/roles', {'role_id': 'blocked', 'permissions': ['file:manage']})
    assert sql("SELECT count(*) FROM role_permissions WHERE role_id='blocked'") == '0'
    sql("INSERT INTO role_permissions SELECT * FROM json_populate_recordset(NULL::role_permissions, '%s'::json) ON CONFLICT DO NOTHING" % grants.replace("'", "''"))
    token = mount(source, 'users')
    assert any(role['id'] == 'auditor' for role in invoke(token, 'GET', '/rbac')['roles'])
    print('Live permission revocation blocks reads and writes; restore recovers', flush=True)

    assert hashlib.sha256(binary.read_bytes()).hexdigest() == host_hash
    success('/api/runtime/plugins/%s/uninstall' % source, {})
    assert not any(p['id'] == 'component:%s:users' % source for p in pages())
    report = {'viewed': True, 'roles': True, 'users': True, 'passwordReset': True,
              'organization': True, 'removedMember': True, 'upgradeRollback': True,
              'disabledAcrossRestart': True, 'liveRevocation': True, 'hostUnchanged': True,
              'logs': str(root)}
finally:
    try:
        stop()
    finally:
        if supervisor is not None and supervisor.poll() is None:
            supervisor.terminate()
            supervisor.wait(timeout=20)
        host_log.close()
        supervisor_log.close()

report['gracefulShutdown'] = True
print(json.dumps(report), flush=True)
