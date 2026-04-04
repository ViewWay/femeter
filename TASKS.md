# TASKS.md - 自主任务队列

## 规则
- 优先级从上到下执行
- 每个任务完成后标记 [x] 并记录 commit
- 遇到无法自主决策的问题 → 记录到 BLOCKERS.md，继续下一个任务
- 阻塞问题在 BLOCKERS.md 中列出，等用户回复后继续

## 当前任务

### 优先级 P0 — 功能完善
- [ ] IC 类覆盖率提升至 90%+（当前 ~40%，需补蓝皮书剩余 60+ IC 类）
- [ ] Set/Action/Block Transfer 互操作测试
- [ ] 真实电表硬件验证准备（测试用例文档）

### 优先级 P1 — 质量提升
- [ ] comm.rs 补充边界测试
- [ ] storage.rs 掉电恢复压力测试
- [ ] power_manager.rs 低功耗状态机覆盖测试

### 优先级 P2 — 文档
- [ ] API 文档生成 (cargo doc)
- [ ] 部署指南（J-Link 烧录步骤）

## 已完成
- [x] 2026-04-04 CI 修复 (3-job workflow 稳定)
