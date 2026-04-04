#!/usr/bin/env python3
"""测试 SNRM 帧的解析"""

# SNRM 帧内容
snrm_frame = bytes([0x7E, 0xA0, 0x07, 0x03, 0x21, 0x93, 0x0F, 0x01, 0x7E])

print("SNRM Frame:", snrm_frame.hex().upper())
print()

# 手动解析
print("Manual parsing:")
print(f"  Flag (start): 0x{snrm_frame[0]:02X}")
print(f"  Length field: 0x{snrm_frame[1]:02X} (raw), {snrm_frame[1]} (decimal)")
print(f"  Server address: 0x{snrm_frame[2]:02X}")
print(f"  Client address: 0x{snrm_frame[3]:02X}")
print(f"  Control: 0x{snrm_frame[4]:02X}")
print(f"  HCS: 0x{snrm_frame[5]:02X}{snrm_frame[6]:02X}")
print(f"  Flag (end): 0x{snrm_frame[7]:02X}")
print()

# 解析地址
server_addr = snrm_frame[2] >> 1
client_addr = snrm_frame[3] >> 1
print(f"Decoded addresses:")
print(f"  Server logical address: {server_addr}")
print(f"  Client address: {client_addr} (0x{client_addr:02X})")
print()

# 解析控制字段
control = snrm_frame[4]
print(f"Control field: 0x{control:02X}")
if control & 0x03 == 0x03:
    print(f"  Frame type: U-frame")
    if control & 0xEF == 0x83:
        print(f"  Subtype: SNRM")
    elif control & 0xEF == 0x63:
        print(f"  Subtype: UA")
    print(f"  P/F bit: {(control >> 4) & 1}")
print()

# 计算 CRC
def crc16_hdlc(data):
    """Calculate CRC-16/HDLC (ITU-T X.25)"""
    crc = 0xFFFF
    for byte in data:
        crc ^= byte
        for _ in range(8):
            if crc & 1:
                crc = (crc >> 1) ^ 0x8408
            else:
                crc >>= 1
    return crc ^ 0xFFFF

# 验证 HCS (CRC of length + address + control)
hcs_data = snrm_frame[1:5]  # Length + Server + Client + Control
calculated_hcs = crc16_hdlc(hcs_data)
frame_hcs = (snrm_frame[6] << 8) | snrm_frame[5]

print(f"HCS verification:")
print(f"  Data: {hcs_data.hex().upper()}")
print(f"  Calculated HCS: 0x{calculated_hcs:04X}")
print(f"  Frame HCS: 0x{frame_hcs:04X}")
print(f"  Match: {calculated_hcs == frame_hcs}")
print()

# 构造 UA 帧
print("Constructing UA frame:")
# UA frame format: Flag | Length | Server | Client | Control (UA) | HCS | Flag
ua_control = 0x63 | ((control >> 4) & 1) << 4  # UA with same P/F bit
ua_length = 0x07  # Same length as SNRM
ua_data = bytes([ua_length, snrm_frame[2], snrm_frame[3], ua_control])
ua_hcs = crc16_hdlc(ua_data)
ua_frame = bytes([0x7E]) + ua_data + bytes([ua_hcs & 0xFF, (ua_hcs >> 8) & 0xFF]) + bytes([0x7E])

print(f"  UA control: 0x{ua_control:02X}")
print(f"  UA data: {ua_data.hex().upper()}")
print(f"  UA HCS: 0x{ua_hcs:04X}")
print(f"  UA frame: {ua_frame.hex().upper()}")
