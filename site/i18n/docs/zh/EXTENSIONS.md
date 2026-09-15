# 项目扩展

扩展在共享的原生/WASM Rust 运行时中用 Velin 0.3.0 执行。模块是不可变 `input` 的
纯函数，必须以 `perform return(value)` 返回，也可以用 `perform fail(message)` 主动
失败。每次调用都会创建全新的 VM。编译期会拒绝其他宿主命令以及 `random`/`chance`
内置函数；浮点、时钟和文件系统均不可用。每次调用固定最多执行 10,000 条立即指令。
输入和输出还受剧情数据预算限制：最多 4096 个值、16 层集合和 1 MiB 文本。

```json
{"version":2,"modules":{"inventory.reward":"extensions/reward.velin"}}
```

```velin
set next = push(input, "map")
perform return(next)
```

```text
extend bag = "inventory.reward" bag
```

```json
{"type":"extension","text":"Collect reward","name":"inventory.reward","variable":"bag","input":"bag"}
```

manifest v2 只接受 `.velin` 模块。模块源码按运行时编译一次，计入程序指纹，并打进
目录、归档和 Web 构建。编辑模块会改变构建身份。结果参与普通存档和回滚。失败的界面
调用（包括显式 `fail`）不提交任何更改。宿主集成可以调用
`Runtime::invoke_extension` 或 `Runtime::apply_extension_expression`；外部副作用属于
平台适配器，不属于确定性剧情函数。
