// module.exports = (config, env, helpers) => {
//   helpers.getPluginsByName(config, 'HtmlWebpackPlugin').forEach(({ plugin }) => {
//     if (plugin && plugin.userOptions) {
//       plugin.userOptions.template = './src/index.html';
//     } else {
//       console.warn('HtmlWebpackPlugin instance does not have userOptions. Skipping.');
//     }
//   });
// };

// export default (config, env, helpers) => {
//   helpers.getPluginsByName(config, 'HtmlWebpackPlugin').forEach(({ plugin }) => {
//     plugin.options.templateContent = `
//       <!DOCTYPE html>
//       <html lang="en">
//       <head>
//         <meta charset="UTF-8">
//         <meta name="viewport" content="width=device-width, initial-scale=1.0">
//         <title>NIC PWA</title>
//       </head>
//       <body>
//         <div id="root"></div>
//       </body>
//       </html>
//     `;
//   });
// };

// export default (config, env, helpers) => {
//   helpers.getPluginsByName(config, 'HtmlWebpackPlugin').forEach(({ plugin }) => {
//     plugin.options.inject = false; // Disable automatic injection to take control
//     plugin.options.templateContent = ({ htmlWebpackPlugin }) => `
//       <!DOCTYPE html>
//       <html lang="en">
//       <head>
//         <meta charset="UTF-8">
//         <meta name="viewport" content="width=device-width, initial-scale=1.0">
//         <title>NIC PWA</title>
//         ${htmlWebpackPlugin.tags.headTags}
//       </head>
//       <body>
//         <div id="root"></div>
//         ${htmlWebpackPlugin.tags.bodyTags}
//       </body>
//       </html>
//     `;
//   });
// };

export default (config, env, helpers) => {
  config.plugins.forEach((plugin) => {
    if (plugin.constructor.name === 'HtmlWebpackPlugin') {
      plugin.options.template = './template.html';
    }
  });
};