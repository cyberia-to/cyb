# Бенчмарки проекта

## Baseline (Webpack 5.94.0 + Node.js 22)

Дата: 2026-02-19
Коммит: 290ed5fe

### Production build
- **Время:** 101.45s (wall clock 1m42s)
- **Webpack compilation:** 96771ms
- **Размер build/:** 123MB

### Bundle breakdown (top JS chunks)
| Файл | Размер |
|------|--------|
| 943.*.js | 2.9MB |
| 895.*.js | 2.0MB |
| 667.*.js | 1.9MB |
| main-*.js | 1.7MB |
| 203.*.js | 1.5MB |
| 187.*.chunk.js | 1.3MB |
| CSS (main) | 24KB |
| JS файлов всего | 24 |

### Dev server startup
- TODO: замерить при следующем запуске

---

## После Rspack (Фаза 1) — Rspack 5.75.0 + Node.js 22

Дата: 2026-02-19
Коммит: c651ba69

### Production build
- **Время:** 9.55s (wall clock ~10s)
- **Rspack compilation:** 8680ms
- **Размер build/:** 96MB

### Сравнение
| Метрика | Webpack | Rspack | Разница |
|---------|---------|--------|---------|
| Compilation | 96.8s | 8.7s | **11.1x быстрее** |
| Wall clock | 101s | 9.5s | **10.6x быстрее** |
| Размер build/ | 123MB | 96MB | **-22%** |
| JS файлов | 24 | 15 | -37% |

### Warnings (безвредные)
- `libsodium-sumo`: `__dirname` is mocked (4 warnings)

### Временно отключено
- BootloaderPlugin (несовместим с Rspack compilation API)
- WorkboxPlugin/InjectManifest (использует webpack.EntryPlugin внутри)
