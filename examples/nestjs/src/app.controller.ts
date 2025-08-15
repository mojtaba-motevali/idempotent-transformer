import { Body, Controller, Headers, Post } from '@nestjs/common';
import { AppService } from './app.service';

@Controller()
export class AppController {
  constructor(private readonly appService: AppService) {}

  @Post()
  async handlePayment(
    @Headers('Idempotent-Key') key: string,
    @Body() data: { itemId: string; amount: number },
  ): Promise<string> {
    return await this.appService.handlePayment(key, data);
  }
}
