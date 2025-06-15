import { Body, Controller, Headers, Post } from '@nestjs/common';
import { AppService } from './app.service';

@Controller()
export class AppController {
  constructor(private readonly appService: AppService) {}

  @Post()
  async writeJSON(
    @Headers('Idempotent-Key') key: string,
    @Body() data: any,
  ): Promise<string> {
    return await this.appService.writeJSON(key, data);
  }
}
