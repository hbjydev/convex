import { expect, test } from 'vitest';
import { toCsv } from './settings-csv';

test('CSV preserves quoted names and prevents spreadsheet formula execution', () => {
  expect(
    toCsv([
      ['Name', 'Identifier'],
      ['Design, "Systems"', 'DS'],
      ['=HYPERLINK("https://example.com")', 'EQ'],
      ['  +123', 'PLUS'],
      ['@formula', 'AT'],
      ['-42', 'NEG'],
    ]),
  ).toBe(
    '"Name","Identifier"\r\n"Design, ""Systems""","DS"\r\n"\'=HYPERLINK(""https://example.com"")","EQ"\r\n"\'  +123","PLUS"\r\n"\'@formula","AT"\r\n"\'-42","NEG"',
  );
});
