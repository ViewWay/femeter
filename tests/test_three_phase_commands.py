#!/usr/bin/env python3
"""
测试虚拟电表三相控制命令

需要安装: pip install pyserial
"""

import serial
import time
import sys
import subprocess
import os
import signal

def run_shell_commands(commands):
    """启动虚拟电表并发送命令"""
    # 启动虚拟电表
    proc = subprocess.Popen(
        ['./target/release/virtual-meter'],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        cwd='/Users/yimiliya/.openclaw/workspace/femeter'
    )
    
    # 等待启动
    time.sleep(1)
    
    # 发送所有命令
    all_commands = '\n'.join(commands) + '\nquit\n'
    stdout, stderr = proc.communicate(input=all_commands, timeout=10)
    
    return stdout, stderr

def main():
    print("=" * 60)
    print("虚拟电表三相控制命令测试")
    print("=" * 60)
    
    # 测试命令序列
    commands = [
        # 查看帮助
        "help",
        
        # 测试三相组合设置
        "set three-phase 230 5 50 0.95",
        
        # 测试单相设置
        "set ua 235",
        "set ub 228",
        "set uc 232",
        "set ia 5.5",
        "set ib 4.8",
        "set ic 5.2",
        
        # 测试角度设置
        "set angle-a 0",
        "set angle-b -120",
        "set angle-c 120",
        
        # 测试功率因数设置
        "set pf 0.9",
        
        # 查询命令
        "get voltage",
        "get current",
        "get angle",
        "get power",
        "get energy",
        "get frequency",
        "get power-factor",
        
        # 测试异常场景
        "set ua 0",
        "get status-word",
        
        "set ua 280",
        "get status-word",
        
        "set ua 170",
        "get status-word",
        
        "set ia 70",
        "get status-word",
        
        # 恢复正常
        "set three-phase 220 5 50 0.95",
        "get status-word",
        
        # 查看状态
        "status",
    ]
    
    print("\n执行命令序列...")
    stdout, stderr = run_shell_commands(commands)
    
    print("\n" + "=" * 60)
    print("输出结果:")
    print("=" * 60)
    print(stdout)
    
    if stderr:
        print("\n" + "=" * 60)
        print("错误输出:")
        print("=" * 60)
        print(stderr)
    
    print("\n" + "=" * 60)
    print("测试完成!")
    print("=" * 60)

if __name__ == "__main__":
    main()
