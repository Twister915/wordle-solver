const path = require('path');
const fs = require('fs');
const WasmPackPlugin = require('@wasm-tool/wasm-pack-plugin');
const HtmlWebpackPlugin = require('html-webpack-plugin')
const CompressionPlugin = require('compression-webpack-plugin');

const distPath = path.resolve(__dirname, "dist");
const wasmDistPath = path.resolve(__dirname, "dist/wasm")
fs.mkdirSync(distPath, {recursive: true});

console.log(__dirname);

module.exports = (env, argv) => {
    const contentDistPath = path.resolve(distPath, argv.mode === 'production' ? 'prod' : 'dev');

    return {
        devServer: {
            static: contentDistPath,
            compress: argv.mode === 'production',
            historyApiFallback: true,
            port: 8000
        },
        entry: {
            app: './bootstrap.js',
        },
        output: {
            path: contentDistPath,
            filename: "[name].[contenthash].js",
            webassemblyModuleFilename: "app.[hash].wasm",
            publicPath: "/",
        },
        module: {
            rules: [
                {
                    test: /\.s?[ac]ss$/i,
                    use: [
                        'style-loader',
                        'css-loader',
                        'sass-loader',
                    ],
                },
                {
                    test: /\.(png|jpe?g|gif)$/i,
                    use: [
                        {
                            loader: 'file-loader',
                        },
                    ],
                },
            ],
        },
        plugins: [
            new WasmPackPlugin({
                crateDirectory: __dirname,
                extraArgs: "--no-typescript -- . -Z build-std=alloc,panic_abort,std,proc_macro -Z build-std-features=",
                outDir: wasmDistPath,
            }),
            new HtmlWebpackPlugin({
                title: "Wordle Site",
                meta: {
                    viewport: "width=device-width, initial-scale=1"
                }
            }),
            new CompressionPlugin({
                test: /.(js|wasm|html)(\?.*)?$/i,
            })
        ],
        experiments: {
            syncWebAssembly: true,
        }
    };
};