import { CheckSumGenerator } from './checksum-generator';

describe('ChecksumGenerator', () => {
  it('should be defined', () => {
    expect(new CheckSumGenerator()).toBeDefined();
  });

  it('should generate a checksum', async () => {
    const generator = new CheckSumGenerator();
    const checksum = await generator.generate(
      JSON.stringify({
        a: 1,
        b: 2,
        c: 3,
      })
    );
    expect(checksum).toEqual(785669035);
  });
});
