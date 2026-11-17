# A03-fork706 人工检查

预计 5 分钟；最重要的是确认免确认规则只命中独立的、明确指向 `origin` 分支的普通推送。

1. 在任务的 Evidence 中查看临时 CLI 验证结果：普通显式分支推送没有权限提示，但闭合的本地测试端点阻止了实际推送；裸 `git push`、没有分支名的 `git push origin` 和强推都弹出了确认，且没有获批。
2. 打开 `crates/monitor/src/terminal.rs`，确认允许规则是完整匹配的 `git push origin refs/heads/<branch>`，并且只注入到监测启动的安静 Antigravity 会话；确认裸推送、危险选项和删除 refspec 都有询问规则。
3. 检查 `crates/monitor/src/request.rs` 的安静模式说明，确认它要求代理使用单独的显式分支推送，并把组合命令留在确认范围内。
4. 如需复跑权限行为，只在 `~/allomos-tests/A03-fork706-permissions/local-repo` 的闭合 loopback 测试仓库中操作；确认不会连接真实远端。对每个确认提示选择取消。

```sh
env -u ALLOMOS_TASKBAG cargo test -p allomos-monitor
git config --get remote.origin.url
```

预期：测试为 71 项通过、0 项失败、1 项忽略；远端地址为 `http://127.0.0.1:1/a03-fork706.git`。不要在真实远端验证推送。
