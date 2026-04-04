# AUTOWORK.md - 自治工作流配置

## 核心原则
1. **不等用户** — 能自主决策就自主决策
2. **分支隔离** — 每个任务一个 feature 分支，CI 通过后开 PR
3. **PR 合并** — 用 `gh pr create --fill` 创建 PR，CI 通过后 `gh pr merge --squash`
4. **本地先验** — push 前必须跑 cargo check + clippy + test
5. **CI 循环** — CI 失败就自动读日志、修复、重推，最多 5 轮
6. **5轮失败** — 记录到 BLOCKERS.md，转下一个任务
7. **资源意识** — Pro 套餐: ~400 prompts/5h, 每prompt ~15-20次调用，抓紧用别浪费

## 分支命名
- feat/<简短描述>
- fix/<简短描述>
- test/<简短描述>

## 工作循环
```
1. 读 TASKS.md，取第一个未完成任务
2. git checkout -b feat/xxx
3. 编码
4. 本地验证:
   - cargo +nightly check --target thumbv6m-none-eabi
   - cargo clippy --all-targets
   - cargo test --workspace
   - cargo fmt --check
5. git add + commit + push origin feat/xxx
6. 等 CI (poll GitHub Actions)
7. CI 结果:
   - PASS → 自 review diff → gh pr create --fill → gh pr merge --squash → 更新 TASKS.md
   - FAIL → 读 CI 日志 → 修复 → goto 4（最多5轮）
8. 回到步骤 1
```

## 自主决策边界
**可以自主决定:**
- 代码风格、命名、模块划分
- 测试用例编写
- Bug 修复方案
- CI 配置调整
- 文档更新

**必须问用户 (写 BLOCKERS.md):**
- 架构方向变更（如换 RTOS、换通信协议）
- 新增外部依赖（引入新 crate）
- 删除已有功能
- 发布版本号

## 用户回复 BLOCKERS 后
- 立即恢复对应任务
- 继续自治循环
