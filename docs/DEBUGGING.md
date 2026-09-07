# 剧情调试与路线回归

`renrs-debug` 使用无窗口 Runtime，可以在没有 GPU、音频设备或播放器设置的环境中运行。
所有命令接受项目目录或 `.renrs` 归档，先执行统一项目检查。

## 查看与录制

```sh
cargo run --bin renrs-debug -- inspect demo
cargo run --bin renrs-debug -- record demo first-route.json
cargo run --bin renrs-debug -- replay demo first-route.json
```

`inspect` 推进到第一个交互点，输出当前源位置、章节、变量、调用栈、等待态和历史长度。
`record` 在终端显示对白和选项；输入 `next` 或回车推进，输入从 1 开始的选项编号做选择，
`state` 查看状态，`quit` 取消。只有到达结局才生成文件，已有文件不会被覆盖。

录制结果保存项目 ID、内容指纹、从 0 开始的选择下标、稳定选项 ID、结局标签和最终变量。
重放时检查这些约束；内容变更后应重新录制。手写路线可以省略 `fingerprint`，以状态断言作为
跨版本回归依据。`choice_ids` 如果提供，长度必须与 `choices` 一致。

## 路线断言

`renrs-init` 的模板包含可直接执行的 `routes.json`：

```json
{
  "routes": [
    {
      "name": "answer",
      "choices": [0],
      "expect_label": "answer",
      "expect_variables": { "trust": 1 },
      "expect_dialogue": "Someone answers"
    },
    {
      "name": "wait",
      "choices": [1],
      "expect_label": "wait",
      "expect_variables": { "trust": 0 }
    }
  ]
}
```

```sh
cargo run --bin renrs-debug -- test my-story my-story/routes.json
```

`expect_variables` 检查指定变量的最终值；`expect_dialogue` 检查历史中是否包含给定文本。
结局标签来自最后实际执行的指令，不会因 `return` 越过标签边界而误报。缺少选择、多余选择、
断言失败、指纹不一致或超过 10,000 次交互都会返回非零退出码。库 API `run_route` 可以显式
设置更大的交互上限。除交互式 `record` 外，调试命令使用 [v1 机器协议](MACHINE_PROTOCOL.md)，
路线结果包含本次实际访问的稳定指令 ID。

## 有界分支探索

```sh
cargo run --bin renrs-debug -- explore my-story
cargo run --bin renrs-debug -- explore my-story --max-runs 256 --max-steps 20000 --max-depth 48
```

默认最多运行 128 条路线、每条 10,000 次交互、选择深度 32。报告包含到达的结局、指令覆盖、
运行错误和被截断的分支。达到上限时 `complete` 为 `false`，命令以失败退出；这不等价于剧情有错，
也不等价于所有分支已被验证。循环和组合爆炸项目应使用明确的路线断言。

这些工具验证剧情状态，不验证字形、图片显示、动画流畅度或实际听感。原生窗口验收见
[规模与运行验证](VALIDATION.md)。
