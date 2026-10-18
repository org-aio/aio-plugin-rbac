# AIO 用户与权限插件 / AIO User & Permission Plugin

作为独立的 v2 Sidecar 整包交付用户、角色、部门和岗位管理。后端只通过宿主
`/identity/invoke` 受控通道读写权限数据，不持有数据库连接或宿主身份；前端是
独立的 Dioxus 页面，通过宿主 `aioPlugin` 桥调用。页面在“系统 / 系统管理”下贡献
用户、角色、部门和岗位四个入口。

Delivered as a standalone v2 Sidecar package providing user, role, department and
post management. The backend only reads and writes through the host-controlled
`/identity/invoke` channel and holds no database connection or host identity; the
frontend is a standalone Dioxus page calling the host `aioPlugin` bridge. It
contributes user, role, department and post entries under System / System Admin.

## 能力 / Capabilities

- `rbac_provider`：宿主受控的权限读写；每次调用由宿主按会话复核 `rbac:manage`。

## 构建 / Build

```sh
sh scripts/build.sh          # 仅前端 / frontend only
sh scripts/build.sh --process # 前端 + 进程产物 / frontend and process artifact
```
