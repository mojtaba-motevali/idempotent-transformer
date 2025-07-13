import { Get, Controller, Inject } from '@nestjs/common';
import { IdempotentStateStore } from '@idempotent-transformer/core';
import { IDEMPOTENT_STORAGE_SERVICE } from './idempotent.constant';

@Controller('idempotent')
export class IdempotentController {
  constructor(
    @Inject(IDEMPOTENT_STORAGE_SERVICE)
    private readonly idempotentStorageService: IdempotentStateStore
  ) {}

  @Get('clean-expired')
  async cleanExpired() {
    await this.idempotentStorageService.cleanExpired();
  }
}
