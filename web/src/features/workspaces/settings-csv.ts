export function toCsv(rows: string[][]) {
  return rows
    .map((row) =>
      row
        .map((value) => {
          // Keep names supplied by users from becoming spreadsheet formulas.
          const safe = /^[\s]*[=+@-]|^[\t\r\n]/.test(value)
            ? `'${value}`
            : value;
          return `"${safe.replaceAll('"', '""')}"`;
        })
        .join(','),
    )
    .join('\r\n');
}

export function exportCsv(filename: string, rows: string[][]) {
  const url = URL.createObjectURL(
    new Blob([toCsv(rows)], { type: 'text/csv;charset=utf-8' }),
  );
  const link = document.createElement('a');
  link.href = url;
  link.download = filename;
  link.click();
  URL.revokeObjectURL(url);
}
