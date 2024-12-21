const { defineConfig } = require("@vue/cli-service");

module.exports = defineConfig({
  transpileDependencies: true,
  lintOnSave: false,
  // runtimeCompiler: true, // Avoids eval-like behavior in templates

  chainWebpack: (config) => {
    // fixed nonce to be used throughout the app
    const nonce = 'fixed-nonce-lagarto'; 

    // Inject nonce into script tags
    config.plugin('html').tap((args) => {
      const htmlOptions = args[0];
      htmlOptions.cspNonce = nonce;
      return args;
    });

    // Add a nonce attribute to dynamically injected scripts
    config.module
      .rule('vue')
      .use('vue-loader')
      .tap((options) => {
        options.compilerOptions = {
          ...options.compilerOptions,
          // Ensure script and style tags have nonce
          nodeTransforms: [
            (node) => {
              if (node.tag === 'script' || node.tag === 'style') {
                node.props.push({
                  name: 'nonce',
                  value: `"${nonce}"`,
                });
              }
            },
          ],
        };
        return options;
      });
  },

  devServer: {
  headers: {
    'Content-Security-Policy': [
      "default-src 'self'",
      "script-src 'self' 'nonce-fixed-nonce-lagarto' 'unsafe-eval'",
      "style-src 'self' 'nonce-fixed-nonce-lagarto' 'unsafe-inline'",
      "connect-src 'self' ws://172.20.189.10:8082/ws ws://localhost",
      "img-src 'self' data:",
      "font-src 'self'",
      "object-src 'none'",
      "base-uri 'self'",
      "form-action 'self'",
      ].join('; '), // Combine the policy into a single string
    },
  },
});