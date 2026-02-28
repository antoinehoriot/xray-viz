import { injectable } from 'inversify';
import { Logger } from './logger';

export function greet(name: string): string {
    return `Hello, ${name}!`;
}

export const VERSION = '1.0.0';

export class MyService {
    private logger: Logger;

    constructor(logger: Logger) {
        this.logger = logger;
    }

    greet(name: string): string {
        this.logger.log(`Greeting ${name}`);
        return greet(name);
    }
}

export default MyService;
