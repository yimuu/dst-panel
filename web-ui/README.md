# DST Admin Web UI

Vue 3, TypeScript, and Vite frontend source for DST Admin. Production builds write to the repository root `dist/` directory so the Rust server can keep serving static frontend files with its existing routes.

## Project Setup

```sh
npm install
```

### Compile and Hot-Reload for Development

```sh
npm run dev
```

### Type-Check, Compile and Minify for Production

```sh
npm run build
```

### Run Unit Tests with [Vitest](https://vitest.dev/)

```sh
npm run test:unit
```

### Lint with [ESLint](https://eslint.org/)

```sh
npm run lint
```
