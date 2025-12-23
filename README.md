# Jokkerin Ventti 🏋️

A workout interval timer app for Jokkerin Ventti.

[![Deploy to GitHub Pages](https://github.com/LVK-96/jokkerin-ventti/actions/workflows/deploy.yml/badge.svg)](https://github.com/LVK-96/jokkerin-ventti/actions/workflows/deploy.yml)

**[▶️ Launch App](https://lvk-96.github.io/jokkerin-ventti/)**

## Development

### Prerequisites

- Node.js 20+

### Setup

```bash
npm install
npm run dev
```

Open <http://localhost:5173/jokkerin-ventti/>

### Build

```bash
npm run build
```

Output is in `dist/`.

### Customizing Workouts

Edit `public/Workouts/jokkeri_ventti.json`:

```json
{
  "exercises": [
    {
      "name": "Exercise Name",
      "workoutTime": 40,
      "setCount": 2,
      "pauseTime": 20,
      "intermediateBeeps": [30, 20, 10]  // optional
    }
  ]
}
```

## Tech Stack

- TypeScript
- Vite
- GitHub Pages (via GitHub Actions)

## License

MIT
