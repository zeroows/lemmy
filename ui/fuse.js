const {
  FuseBox,
  Sparky,
  EnvPlugin,
  CSSPlugin,
  WebIndexPlugin,
  QuantumPlugin,
  BannerPlugin,
} = require('fuse-box');
const transformInferno = require('ts-transform-inferno').default;
const transformClasscat = require('ts-transform-classcat').default;
let fuse, app;
let isProduction = false;
let libreJsLicense =
  '// @license magnet:?xt=urn:btih:0b31508aeb0634b347b8270c7bee4d411b5d4109&dn=agpl-3.0.txt GNU Affero General Public License, version 3';

Sparky.task('config', _ => {
  fuse = new FuseBox({
    homeDir: 'src',
    hash: isProduction,
    output: 'dist/$name.js',
    experimentalFeatures: true,
    cache: !isProduction,
    sourceMaps: !isProduction,
    transformers: {
      before: [transformClasscat(), transformInferno()],
    },
    alias: {
      locale: 'moment/locale',
    },
    plugins: [
      EnvPlugin({ NODE_ENV: isProduction ? 'production' : 'development' }),
      CSSPlugin(),
      WebIndexPlugin({
        title: 'Inferno Typescript FuseBox Example',
        template: 'src/index.html',
        path: isProduction ? '/static' : '/',
      }),
      isProduction &&
        QuantumPlugin({
          bakeApiIntoBundle: 'app',
          treeshake: true,
          uglify: true,
        }),
      BannerPlugin(libreJsLicense),
    ],
  });
  app = fuse.bundle('app').instructions('>index.tsx');
});
Sparky.task('clean', _ => Sparky.src('dist/').clean('dist/'));
Sparky.task('env', _ => (isProduction = true));
Sparky.task('copy-assets', () =>
  Sparky.src('assets/**/**.*').dest(isProduction ? 'dist/' : 'dist/static')
);
Sparky.task('dev', ['clean', 'config', 'copy-assets'], _ => {
  fuse.dev({
    fallback: 'index.html',
  });
  app.hmr().watch();
  return fuse.run();
});
Sparky.task('prod', ['clean', 'env', 'config', 'copy-assets'], _ => {
  return fuse.run();
});
