import { IdempotentStateStore } from '@idempotent-transformer/core';
import { Client, ClientConfig } from 'pg';

export class PostgresAdapter implements IdempotentStateStore {
  private client: Client;
  private connected = false;
  private tableName = 'IdempotentState';
  constructor(options: string | ClientConfig) {
    this.client = new Client(options);
  }

  async connect() {
    if (this.connected) {
      return;
    }
    await this.client.connect();
    this.connected = true;
    await this.client.query(
      `CREATE TABLE IF NOT EXISTS ${this.tableName} (workflowId VARCHAR(255), taskId VARCHAR(255), value BYTEA, expireAt TIMESTAMP DEFAULT NULL)`
    );
  }

  async disconnect() {
    if (!this.connected) {
      return;
    }
    await this.client.end();
    this.connected = false;
  }

  async isConnected() {
    return this.connected;
  }

  async save({
    workflowId,
    taskId,
    value,
  }: {
    workflowId: string;
    taskId: string;
    value: Uint8Array<ArrayBufferLike>;
  }): Promise<void> {
    const buffer = value instanceof Buffer ? value : Buffer.from(value);
    await this.client.query(
      `INSERT INTO ${this.tableName} (workflowId, taskId, value) VALUES ($1, $2, $3)`,
      [workflowId, taskId, buffer]
    );
  }

  async findAll(
    workflowId: string
  ): Promise<{ workflowId: string; taskId: string; value: Uint8Array<ArrayBufferLike> }[]> {
    const { rows } = await this.client.query(
      `SELECT workflowId, taskId, value FROM ${this.tableName} WHERE workflowId = $1`,
      [workflowId]
    );
    return rows;
  }

  async find({
    workflowId,
    taskId,
  }: {
    workflowId: string;
    taskId: string;
  }): Promise<{ workflowId: string; taskId: string; value: Uint8Array<ArrayBufferLike> } | null> {
    const { rows } = await this.client.query(
      `SELECT workflowId, taskId, value FROM ${this.tableName} WHERE workflowId = $1 AND taskId = $2`,
      [workflowId, taskId]
    );
    return rows.length > 0 ? rows[0] : null;
  }

  async complete(workflowId: string, expiredAt: number): Promise<void> {
    const expireDate = new Date(expiredAt);
    await this.client.query(`UPDATE ${this.tableName} SET expireAt = $1 WHERE workflowId = $2`, [
      expireDate,
      workflowId,
    ]);
  }

  async cleanExpired(): Promise<void> {
    await this.client.query(`DELETE FROM ${this.tableName} WHERE expireAt < NOW()`);
  }
}
