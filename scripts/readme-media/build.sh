#!/usr/bin/env bash
# Regenera las capturas y los GIF del README (docs/media/). Ver README.md de esta carpeta.
set -euo pipefail
cd "$(dirname "$0")"
ROOT=../..
MEDIA=$ROOT/docs/media
PORT=${PORT:-5304}
mkdir -p "$MEDIA"

[ -d scans ] || node make-scans.mjs

# Servidor de Vite con la interfaz real; se detiene al salir.
(cd "$ROOT" && npx vite --port "$PORT" --strictPort >/dev/null 2>&1) &
VITE=$!
trap 'kill $VITE 2>/dev/null; fuser -k -n tcp $PORT >/dev/null 2>&1 || true' EXIT
until curl -sf "http://localhost:$PORT/scripts/readme-media/app.html" >/dev/null; do sleep 0.5; done

for lang in en es; do
  node capture.mjs "$lang" "$PORT"
  out=out/$lang
  # PNG: máximo 1600 px de ancho y pngquant.
  for shot in queue result pages agents; do
    ffmpeg -loglevel error -y -i "$out/$shot.png" -vf "scale='min(1600,iw)':-2:flags=lanczos" "$out/$shot-s.png"
    pngquant --quality 70-90 --force --output "$MEDIA/$shot-$lang.png" "$out/$shot-s.png"
  done
  # GIF: cuadros con su duración (concat de ffmpeg), 900 px, 12 fps, paleta en dos pasadas.
  node -e '
    const f = require("./'"$out"'/hero/frames.json");
    let s = f.map(([n, d]) => `file '"'"'${n}'"'"'\nduration ${d}`).join("\n");
    s += `\nfile '"'"'${f[f.length - 1][0]}'"'"'\n`;
    require("fs").writeFileSync("./'"$out"'/hero/list.txt", s);'
  VF="fps=12,scale=900:-1:flags=lanczos"
  ffmpeg -loglevel error -y -f concat -safe 0 -i "$out/hero/list.txt" -vf "$VF,palettegen=stats_mode=diff" "$out/palette.png"
  ffmpeg -loglevel error -y -f concat -safe 0 -i "$out/hero/list.txt" -i "$out/palette.png" \
    -lavfi "$VF [x]; [x][1:v] paletteuse=dither=bayer:bayer_scale=5:diff_mode=rectangle" -loop 0 "$MEDIA/hero-$lang.gif"
done
ls -la "$MEDIA"
