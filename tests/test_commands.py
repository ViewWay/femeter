#!/usr/bin/env python3
"""测试虚拟电表三相控制命令"""
import subprocess
import time
import sys

def run_commands(commands):
    """启动虚拟电表并运行命令"""
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
    
    # 发送命令
    for cmd in commands:
        proc.stdin.write(cmd + '\n')
        time.sleep(0.3)
    
    # 读取输出
        output, proc.stdout.read()
        err = proc.stderr.read()
        if err:
            print(f"错误: {err}")
        if output:
            print(f"输出:\n{output}")
            print("-" * 40)
    
    # 关闭进程
    proc.terminate()
    time.sleep(0.5)
    
    return output, err

if __name__ == "__main__":
    commands = [
                "help",
                "set three-phase 230 5 50 0.95",
                "set ua 230",
                "set ub 230", 
                "set uc 230",
                "set ia 5",
                "set ib 5",
                "set ic 5",
                "set angle-a 0",
                "set angle-b -120",
                "set angle-c 120",
                "set freq 50",
                "set pf 0.95",
                "get voltage",
                "get current",
                "get angle",
                "get power",
                "get energy",
                "get frequency",
                "get power-factor",
                "get status-word",
                # 异常场景
                "set ua 0",
                "set ub 0",
                "set uc 0",
                "get status-word",
                "set angle-a 180",
                "get status-word",
                "quit",
            ]
            
            print(f"运行 {len(commands)} 条命令")
            return output, err
        finally:
            print("✅ 测试完成")
            sys.exit(0)
        else:
            print(f"❌ 测试失败: {e}")
            sys.exit(1)

if __name__ == "__main__":
    run_commands(commands)
