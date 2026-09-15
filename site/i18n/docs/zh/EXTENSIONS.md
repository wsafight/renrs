# 项目扩展

扩展在共享的原生/WASM Rust 运行时中用 Velin 0.4.0 执行。模块是不可变 `input` 的
纯函数，必须以 `perform return(value)` 返回，也可以用 `perform fail(message)` 主动
失败。每次调用都会重置独立的 VM；空闲 VM workspace 可跨调用复用，并发调用不会共享
可变状态。编译期会拒绝其他宿主命令以及 `random`/`chance`
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

## 运行模型

manifest v2 只接受 `.velin` 模块。模块源码按运行时编译一次，计入程序指纹，并打进
目录、归档和 Web 构建。编辑模块会改变构建身份。结果参与普通存档和回滚。失败的界面
调用（包括显式 `fail`）不提交任何更改。宿主集成可以调用
`Runtime::invoke_extension` 或 `Runtime::apply_extension_expression`；外部副作用属于
平台适配器，不属于确定性剧情函数。

每个编译模块惰性保留最多四个空闲 `MachineInvoker`。调用只在取出和归还 invoker 时短暂
锁定池，执行 VM 时不持锁；并发调用耗尽池时，各自获得独立 machine。每个 machine 在绑定
`input` 前都会重启到已验证的初始 frame，因此成功、失败和并发调用都不会相互泄漏状态。

RenRS 保存自有的 `MachineInvoker`，而不是借用模块的 `PureModuleInvoker<'_>`，从而避免
自引用结构，同时保留 `Extensions::invoke(&self)`、可克隆与并发使用。输入 slot 和
`return` / `fail` host ID 只解析一次；单参数 host 路径避免每次构造输入 map 和单元素
host vector。Rhai 到 Velin 的实测结果和复现命令见
[性能测量](PERFORMANCE.md#velin-04-扩展边界)。
