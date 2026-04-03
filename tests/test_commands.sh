#!/bin/bash
cd /Users/yimiliya/.openclaw/workspace/femeter

# 启动虚拟电表（非交互模式，./target/release/virtual-meter --non-interactive &
PID=$!

# 等待启动
sleep 2

# 发送命令
echo "set three-phase 230 5 50 0.95" > /proc/pt/$PID
sleep 1

echo "get voltage" > /proc/pt/$PID
sleep 1

echo "get power" > /proc/pt/$PID
sleep 1

echo "get status-word" > /proc/pt/$PID
sleep 1

# 模拟异常场景
echo "set ua 0" > /proc/pt/$PID
sleep 1

echo "get status-word" > /proc/pt/$PID
sleep 1

echo "set ub 0 uc 0" > /proc/pt/$PID
sleep 1

echo "get status-word" > /proc/pt/$PID
sleep 1

# 退出
echo "quit" > /proc/pt/$PID

# 等待进程结束
wait 2

# 显示输出
cat /proc/$PID/fd
