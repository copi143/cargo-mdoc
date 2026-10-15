import path from "path";

export default {
  plugins: {
    "postcss-mixins": {
      mixinsDir: path.join(import.meta.dirname, "src/styles/mixins"),
    },
    "postcss-nesting": {},
    autoprefixer: {},
  },
};
