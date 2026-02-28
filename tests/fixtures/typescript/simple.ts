import React, { useState, useEffect } from 'react';
import { format } from './utils';
import type { Config } from '../types';
import * as fs from 'fs';

const LazyModule = import('./lazy-module');

function main(): void {
    const [count, setCount] = useState(0);
    const value = format(count.toString());
    console.log(value);
}

const helper = (x: number) => x * 2;
