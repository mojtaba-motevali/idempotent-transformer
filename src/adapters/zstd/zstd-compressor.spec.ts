import { ZstdCompressor } from './zstd-compressor';

describe('ZstdCompressor', () => {
  it('should compress and decompress data', async () => {
    const compressor = new ZstdCompressor();
    const data = new Uint8Array([1, 2, 3, 4, 5]);
    const compressed = await compressor.compress(data);
    const decompressed = await compressor.decompress(compressed);
    expect(new Uint8Array(decompressed)).toEqual(data);
  });
});
