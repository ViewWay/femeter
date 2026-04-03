#!/usr/bin/env python3
"""Connect to FeMeter virtual meter via DLMS/HDLC over TCP using dlms-cosem"""

import sys, os
sys.path.insert(0, "/Users/yimiliya/.openclaw/workspace/dlms-cosem")
os.environ.setdefault("STRUCTLOG_LOGLEVEL", "INFO")

import structlog
structlog.configure(logger_factory=structlog.PrintLoggerFactory())

from dlms_cosem import cosem, enumerations
from dlms_cosem.client import DlmsClient
from dlms_cosem.io import HdlcTransport, BlockingTcpIO
from dlms_cosem.security import NoSecurityAuthentication


def main():
    tcp = BlockingTcpIO(host="127.0.0.1", port=4059)
    
    transport = HdlcTransport(
        client_logical_address=0x10,
        server_logical_address=0x01,
        io=tcp,
    )
    
    auth = NoSecurityAuthentication()
    
    client = DlmsClient(
        transport=transport,
        authentication=auth,
    )
    
    try:
        print("1. HDLC connect...")
        transport.connect()
        print("   Connected!")
        
        print("2. DLMS associate...")
        client.associate()
        print("   Associated!")
        
        print("3. Reading voltage A (1.0.32.7.0.255)...")
        voltage_attr = cosem.CosemAttribute(
            interface=enumerations.CosemInterface.DATA,
            instance=cosem.Obis(1, 0, 32, 7, 0, 255),
            attribute=2,
        )
        result = client.get(voltage_attr)
        print(f"   Result: {result}")
        
    except Exception as e:
        print(f"Error: {e}")
        import traceback
        traceback.print_exc()
    finally:
        try:
            client.release_association()
        except:
            pass
        try:
            transport.disconnect()
        except:
            pass
        print("Done.")


if __name__ == "__main__":
    main()
