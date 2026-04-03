#!/usr/bin/env python3
import subprocess
import sys
import time

def test_command(cmd):
    """执行单个命令并获取输出"""
    proc = subprocess.Popen(
        ['./target/release/virtual-meter'],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        cwd='/Users/yimiliya/.openclaw/workspace/femeter'
    )
    
    time.sleep(1)  # 等待启动
    
    # 发送命令
    proc.stdin.write(cmd + '\n')
    proc.stdin.write('quit\n')
    proc.stdin.flush()
    
    # 读取输出
    stdout, stderr = proc.communicate(timeout=5)
    
    return stdout, stderr

def main():
    print("=" * 60)
    print("虚拟电表三相控制命令测试")
    print("=" * 60)
    
    # 测试三相组合设置
    print("\n1. 测试: set three-phase 230 5 50 0.95")
    stdout, stderr = test_command("set three-phase 230 5 50 0.95")
    print("输出:", [line for line in stdout.split('\n') if '三相设置' in line or 'angle' in line.lower()])
    
    # 测试查询电压
    print("\n2. 测试: get voltage")
    stdout, stderr = test_command("get voltage")
    print("输出:", [line for line in stdout.split('\n') if '电压' in line or 'A' in line])
    
    # 测试查询功率
    print("\n3. 测试: get power")
    stdout, stderr = test_command("get power")
    print("输出:", [line for line in stdout.split('\n') if '功率' in line or 'W' in line])
    
    # 测试状态字（正常）
    print("\n4. 测试: get status-word (正常)")
    stdout, stderr = test_command("get status-word")
    print("输出:", [line for line in stdout.split('\n') if '状态字' in line or '正常' in line or '⚠' in line])
    
    # 测试异常场景 - A相失压
    print("\n5. 测试: set ua 0 + get status-word (A相失压)")
    stdout, stderr = test_command("set ua 0\nget status-word")
    print("输出:", [line for line in stdout.split('\n') if '失压' in line or '0x' in line])
    
    # 测试异常场景 - A相过压
    print("\n6. 测试: set ua 280 + get status-word (A相过压)")
    stdout, stderr = test_command("set ua 280\nget status-word")
    print("输出:", [line for line in stdout.split('\n') if '过压' in line or '0x' in line])
    
    # 测试功率因数设置
    print("\n7. 测试: set pf 0.9")
    stdout, stderr = test_command("set pf 0.9")
    print("输出:", [line for line in stdout.split('\n') if '功率因数' in line or '角度' in line])
    
    print("\n" + "=" * 60)
    print("✅ 测试完成!")
    print("=" * 60)

if __name__ == "__main__":
    try:
        main()
    except Exception as e:
        print(f"❌ 测试失败: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)
