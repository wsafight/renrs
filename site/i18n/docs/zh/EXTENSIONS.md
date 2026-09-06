# 项目扩展

扩展在共享的原生/WASM Rust 运行时中用 Rhai 执行。模块是不可变 `input` 的纯函数，
最后一个表达式作为剧情数据返回。每次调用都是新作用域。不能 import、eval、使用浮点、
时钟或文件系统。预算：100,000 次操作、32 层调用、4096 个集合项和 1 MiB 字符串；
返回数据还要通过剧情值预算。

```json
{"version":1,"modules":{"inventory.reward":"extensions/reward.rhai"}}
```

```rhai
let next = input;
next.push("map");
next
```

```text
extend bag = "inventory.reward" bag
```

```json
{"type":"extension","text":"Collect reward","name":"inventory.reward","variable":"bag","input":"bag"}
```

模块源码按运行时编译一次，计入程序指纹，并打进目录/归档/Web 构建。编辑模块会改变
构建身份。结果参与普通存档和回滚。失败的界面调用不提交任何更改。宿主集成可以调用
`Runtime::invoke_extension` 或 `Runtime::apply_extension_expression`；外部副作用属于
平台适配器，不属于确定性剧情函数。
