//! 最小 PNG 生成工具（用于图片 fixture）。

/// 计算给定数据的 CRC-32（IEEE / 以太网多项式）。
fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    crc ^ 0xFFFF_FFFF
}

/// 构建 PNG 块：4 字节长度 + 4 字节类型 + 数据 + 4 字节 CRC。
fn png_chunk(chunk_type: [u8; 4], data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(12 + data.len());
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(&chunk_type);
    out.extend_from_slice(data);
    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(&chunk_type);
    crc_input.extend_from_slice(data);
    out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
    out
}

/// 计算给定数据的 Adler-32 校验和。
fn adler32(data: &[u8]) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in data {
        a = (a + u32::from(byte)) % 65_521;
        b = (b + a) % 65_521;
    }
    (b << 16) | a
}

/// 创建最小有效 1x1 红色像素 PNG（RGB，8 位深度）。
///
/// 图像由原始 IHDR/IDAT/IEND 块构造，使用 deflate "stored" 块（无压缩），
/// 因此不需要外部 crate。
pub(super) fn create_red_png() -> Vec<u8> {
    let mut png = Vec::with_capacity(128);

    // PNG 8 字节签名
    png.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);

    // IHDR: width=1, height=1, bit_depth=8, color_type=2 (RGB),
    //       compression=0, filter=0, interlace=0
    let ihdr_data: Vec<u8> = [
        1u32.to_be_bytes().as_slice(), // width
        1u32.to_be_bytes().as_slice(), // height
        [0x08_u8, 0x02, 0x00, 0x00, 0x00].as_slice(),
    ]
    .concat();
    png.extend_from_slice(&png_chunk(*b"IHDR", &ihdr_data));

    // IDAT: zlib-wrapped deflate stored block containing one scanline.
    //   Scanline = [filter=None(0x00), R=0xFF, G=0x00, B=0x00]
    let raw_scanline: [u8; 4] = [0x00, 0xFF, 0x00, 0x00];
    let adler = adler32(&raw_scanline);
    let idat_data: Vec<u8> = [
        [0x78_u8, 0x01].as_slice(),       // zlib header (deflate, no dict)
        [0x01].as_slice(),                // BFINAL=1, BTYPE=00 (stored)
        4u16.to_le_bytes().as_slice(),    // LEN = 4
        (!4u16).to_le_bytes().as_slice(), // NLEN = bitwise complement of LEN
        raw_scanline.as_slice(),          // literal bytes
        adler.to_be_bytes().as_slice(),   // Adler-32 checksum
    ]
    .concat();
    png.extend_from_slice(&png_chunk(*b"IDAT", &idat_data));

    // IEND (empty data)
    png.extend_from_slice(&png_chunk(*b"IEND", &[]));

    png
}
