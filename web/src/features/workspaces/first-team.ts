export function firstTeam(name: string) {
  const letters = name
    .normalize('NFKD')
    .replace(/[\u0300-\u036f]/g, '')
    .toUpperCase()
    .replace(/[^A-Z]/g, '');
  return { name, key: letters ? letters.slice(0, 3).padEnd(2, 'X') : 'TEAM' };
}
