#!/usr/bin/env python3
"""测试虚拟电表 HDLC SNRM/UA 连接"""

import sys
sys.path.insert(0, "/Users/yimiliya/.openclaw/workspace/dlms-cosem")

from dlms_cosem.io import HdlcTransport, BlockingTcpIO
from dlms_cosem.security import NoSecurityAuthentication
from dlms_cosem.client import DlmsClient

def test_connection():
    """测试 HDLC 连接和基本读取"""
    print("Connecting to virtual meter at 127.0.0.1:4059...")
    
    tcp = BlockingTcpIO(host="127.0.0.1", port=4059)
    transport = HdlcTransport(
        client_logical_address=0x10,
        server_logical_address=0x01,
        io=tcp
    )
    auth = NoSecurityAuthentication()
    client = DlmsClient(transport=transport, authentication=auth)
    
    try:
        # 1. HDLC 连接 (SNRM/UA 握手)
        print("Step 1: HDLC connect (SNRM/UA handshake)...")
        transport.connect()
        print("✓ HDLC connected!")
        
        # 2. DLMS 关联 (AARQ/AARE)
        print("\nStep 2: DLMS associate (AARQ/AARE)...")
        client.associate()
        print("✓ Associated!")
        
        # 3. 读取电压
        print("\nStep 3: Read voltage (1.0.32.7.0.255)...")
        from dlms_cosem import cosem, enumerations
        v = cosem.CosemAttribute(
            interface=enumerations.CosemInterface.DATA,
            instance=cosem.Obis(1, 0, 32, 7, 0, 255),
            attribute=2
        )
        result = client.get(v)
        print(f"✓ Voltage: {result}")
        
        # 4. 断开关联
        print("\nStep 4: Release association...")
        client.release_association()
        print("✓ Association released!")
        
        # 5. 断开连接
        print("\nStep 5: Disconnect...")
        transport.disconnect()
        print("✓ Disconnected!")
        
        print("\n" + "="*50)
        print("SUCCESS: All steps completed!")
        print("="*50)
        return True
        
    except Exception as e:
        print(f"\n✗ ERROR: {e}")
        import traceback
        traceback.print_exc()
        return False
    finally:
        try:
            tcp.close()
        except:
            pass

if __name__ == "__main__":
    success = test_connection()
    sys.exit(0 if success else 1)
