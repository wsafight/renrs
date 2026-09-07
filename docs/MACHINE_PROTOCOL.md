# 机器接口协议

RenRS 的无窗口工具使用版本化 JSON 包装，供 Launcher、编辑器、Agent 和后续 MCP 薄层复用。
当前协议版本为 v1，Schema 位于 [`schemas/machine-protocol-v1.schema.json`](../schemas/machine-protocol-v1.schema.json)。

## 顶层结构

```json
{
  "protocol_version": 1,
  "command": "inspect",
  "ok": true,
  "data": {},
  "diagnostics": [],
  "error": null
}
```

- `protocol_version`：顶层包装与通用诊断结构版本。
- `command`：产生结果的稳定命令标识，例如 `check`、`debug.test`、`accept`、`inspect`、`impact`。
- `ok`：命令是否达到其完成条件。检查失败、路线失败和不完整探索均为 `false`。
- `data`：命令专属结果；失败时仍可保留有意义的部分结果。
- `diagnostics`：带 `code`、严重级别和源码位置的项目诊断。
- `error`：命令级稳定错误码与人类可读消息；成功时为 `null`。

退出码固定为：`0` 成功、`1` 项目/验证/运行失败、`2` 参数错误。消费者必须先检查
`protocol_version`，再根据 `command` 解释 `data`；不能依赖字段顺序或人类可读消息。
v1 允许在对象中增加可选字段，但不会删除或改变现有字段语义。破坏性变化需要提高
`protocol_version`。

## 当前命令

```sh
renrs-check --json <project|archive>
renrs-debug inspect <project|archive>
renrs-debug replay <project|archive> <route.json>
renrs-debug test <project|archive> <routes.json>
renrs-debug explore <project|archive>
renrs-accept <project|archive> [--saves <directory>]
renrs-inspect <project|archive>
renrs-impact <baseline> <candidate>
renrs-impact --git <candidate-directory> [base-ref]
```

`renrs-debug record` 是交互式人工命令，不使用机器包装；其 `state` 输出也仅供终端查看。

## 项目检查

`renrs-inspect` 返回项目 ID、脚本指纹、格式版本、脚本、角色、变量、标签、结局、资源、
本地化状态和路线覆盖。静态可达与路线覆盖是不同字段：前者来自控制流图，后者来自实际重放
`routes.json`。`referenced_by_story` 只表示资源被剧本或进度配置引用，不代表完成视觉验收。

## 影响分析

`renrs-impact` 编译基线和候选项目，再比较标签、路线结果、结局可达与覆盖、本地化以及存档结构
风险。`--git` 从指定 Git revision 读取候选目录对应的已提交文件，默认基线为 `HEAD`；未跟踪文件
不会出现在基线中。

存档风险是保守的静态报告，不是兼容保证。内容指纹变化后，应继续用代表性真实存档运行：

```sh
renrs-accept <candidate> --saves <save-fixtures>
```
