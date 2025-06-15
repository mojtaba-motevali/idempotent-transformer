import { ZstdCompressor } from './zstd-compressor';

describe('ZstdCompressor', () => {
  it('should compress and decompress data', async () => {
    const compressor = new ZstdCompressor();
    const data = new Uint8Array([1, 2, 3, 4, 5]);
    const compressed = await compressor.compress(data);
    const decompressed = await compressor.decompress(compressed);
    expect(decompressed).toEqual(data);
  });

  it('should detect compressed data', () => {
    const compressor = new ZstdCompressor();
    const data = new Uint8Array([0x28, 0xb5, 0x2f, 0xfd, 1, 2, 3, 4, 5]);
    expect(compressor.isCompressed(data)).toBe(true);
  });

  it('should not detect uncompressed data', () => {
    const compressor = new ZstdCompressor();
    const data = new Uint8Array([1, 2, 3, 4, 5]);
    expect(compressor.isCompressed(data)).toBe(false);
  });
});
