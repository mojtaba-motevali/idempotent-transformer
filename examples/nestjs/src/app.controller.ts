import {
  BadRequestException,
  Body,
  Controller,
  Headers,
  Post,
} from '@nestjs/common';
import { AppService } from './app.service';
import { IdempotencyConflictException } from '@idempotent-transformer/core';

@Controller()
export class AppController {
  constructor(private readonly appService: AppService) {}

  @Post()
  async writeJSON(
    @Headers('Idempotent-Key') key: string,
    @Body() data: { id: string; name: string; lastName: string },
  ): Promise<string> {
    try {
      return await this.appService.writeJSON(key, data);
    } catch (err) {
      if (err instanceof IdempotencyConflictException) {
        throw new BadRequestException(err.message);
      }
      throw err;
    }
  }
}
