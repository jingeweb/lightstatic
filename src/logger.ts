import path from 'path';
import { createLogger, format, transports } from 'winston';
import 'winston-daily-rotate-file';
import { LightStaticOptions } from './common';

const { combine, timestamp, printf } = format;

const myFormat = printf(({ message, timestamp }) => {
  return `[${timestamp}] ${message}`;
});

interface Logger {
  info: (message: string) => void;
  error: (message: string) => void;
  access: (message: string) => void;
}

const ConsoleLogger: Logger = {
  info: (message) => console.info(message),
  error: (message) => console.error(message),
  access: (message) => console.info(message),
};

class LoggerWrapper {
  #logger: Logger = ConsoleLogger;
  #accessLogger: Logger = ConsoleLogger;
  #errorLogger: Logger = ConsoleLogger;

  initialize(options: LightStaticOptions) {
    if (!options.logDir) {
      return;
    }
    const opts = {
      maxFiles: '30d',
      datePattern: 'YYYY-MM-DD',
      zippedArchive: false,
      dirname: path.resolve(process.cwd(), options.logDir),
    };
    this.#logger = createLogger({
      level: 'info',
      format: combine(timestamp(), myFormat),
      transports: [new transports.DailyRotateFile({ ...opts, filename: 'info.%DATE%.log', level: 'info' })],
    }) as unknown as Logger;
    this.#errorLogger = createLogger({
      level: 'error',
      format: combine(timestamp(), myFormat),
      transports: [new transports.DailyRotateFile({ ...opts, filename: 'error.%DATE%.log', level: 'error' })],
    }) as unknown as Logger;
    this.#accessLogger = createLogger({
      level: 'info',
      format: combine(timestamp(), myFormat),
      transports: [new transports.DailyRotateFile({ ...opts, filename: 'access.%DATE%.log' })],
    }) as unknown as Logger;
  }
  info(message: string) {
    this.#logger.info(message);
  }
  error(message: string) {
    this.#errorLogger.error(message);
  }
  access(status: number, url: string) {
    const log = 'GET'.yellow + ' ' + (status < 400 ? status.toString().green : status.toString().red) + ' ' + url.cyan;
    this.#accessLogger.info(log);
  }
}
export const logger = new LoggerWrapper();
