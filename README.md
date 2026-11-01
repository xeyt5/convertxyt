# convertxyt

Aplicación de escritorio para descargar videos y audio y convertir archivos, en lotes si hace falta.

## Descargar

Baja la última versión desde [Releases](https://github.com/xeyt5/convertxyt/releases/latest):

- **Linux:** el archivo `.AppImage`. Dale permiso de ejecución (`chmod +x`) y ábrelo.
- **Windows:** el instalador `.exe`. Windows puede mostrar una advertencia porque la app no tiene firma de código de pago; pulsa "Más información" y "Ejecutar de todas formas".

La app avisa sola cuando hay una versión nueva.

Úsala solo con contenido que tengas derecho a descargar.

## Desarrollo

Necesitas Node.js, Rust, las [dependencias de Tauri](https://v2.tauri.app/start/prerequisites/), `yt-dlp` y `ffmpeg` instalados.

```sh
npm ci
npm run tauri dev
```

En desarrollo la app usa el `yt-dlp` y el `ffmpeg` del sistema y no busca actualizaciones.

## Publicar una versión

```sh
npm version patch   # crea el commit y la etiqueta (minor o major para saltos mayores)
git push --follow-tags
```

GitHub Actions compila Linux y Windows, firma los instaladores y publica la versión. Quienes tengan la app verán el aviso la próxima vez que la abran.

Requiere el secreto `TAURI_SIGNING_PRIVATE_KEY` en el repositorio.

## Licencia

[MIT](LICENSE). Los instaladores incluyen programas de terceros con sus propias licencias: consulta [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
