import { createRoot } from "react-dom/client";
import "./index.css";
import App from "./App.jsx";
import {MseRendererProvider} from './context/MseRendererContext';
import mseUpstream from './lib/mse/upstream.mjs';

const mseCorpusUrl = import.meta.env.VITE_MSE_CORPUS_URL
  ?? (import.meta.env.PROD ? new URL(`./mse/${mseUpstream.revision}/`, document.baseURI).href : undefined);

createRoot(document.getElementById("root")).render(
  <MseRendererProvider corpusUrl={mseCorpusUrl} defaultStyle={import.meta.env.VITE_MSE_STYLE}>
    <App />
  </MseRendererProvider>
);
