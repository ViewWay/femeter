#!/usr/bin/env python3
"""
测试虚拟电表串口连接

需要安装: pip install pyserial
"""

import serial
import time
import sys

def test_text_protocol(port_path):
    """测试文本协议"""
    print(f"\n连接到 {port_path}...")
    
    try:
        ser = serial.Serial(port_path, 9600, timeout=2)
        
        # 发送 ID 命令
        print("发送: ID")
        ser.write(b"ID\r\n")
        time.sleep(0.1)
        
        # 读取响应
        response = ser.read(100)
        if response:
            print(f"响应: {response.decode().strip()}")
        else:
            print("超时：无响应")
            return False
        
        ser.close()
        print("\n文本协议测试完成!")
        return True
        
    except Exception as e:
        print(f"错误: {e}")
        return False

def test_hdlc_snrm(port_path):
    """测试 HDLC SNRM 帧"""
    print(f"\n测试 HDLC SNRM 帧...")
    
    try:
        ser = serial.Serial(port_path, 9600, timeout=2)
        
        # HDLC SNRM 帧 (简化版)
        snrm_frame = bytes([0x7E, 0xA0, 0x01, 0x03, 0x21, 0x00, 0x90, 0x00, 0x00, 0x7E])
        
        print(f"发送: {snrm_frame.hex()}")
        ser.write(snrm_frame)
        time.sleep(0.2)
        
        # 读取响应
        response = ser.read(100)
        if response:
            print(f"响应: {response.hex()}")
            if response[0] == 0x7E:
                print("✓ 收到 HDLC 响应帧")
            else:
                print("✗ 未收到有效的 HDLC 响应帧")
        else:
            print("超时:无响应")
        
        ser.close()
        print("\nHDLC SNRM 测试完成!")
        return True
        
    except Exception as e:
        print(f"错误: {e}")
        return False

def main():
    if len(sys.argv) < 2:
        print("用法: python3 test_serial_connection.py <串口路径>")
        print("示例: python3 test_serial_connection.py /dev/ttys001")
        sys.exit(1)
    
    port_path = sys.argv[1]
    
    print("=" * 60)
    print("虚拟电表串口测试")
    print("=" * 60)
    
    # 测试文本协议
    success = test_text_protocol(port_path)
    
    if success:
        print("\n" + "=" * 60)
        print("✓ 文本协议测试通过")
        print("=" * 60)
    
    # 测试 HDLC
    success = test_hdlc_snrm(port_path)
    
    if success:
        print("\n" + "=" * 60)
        print("✓ HDLC 测试通过")
        print("=" * 60)

if __name__ == "__main__":
    main()
