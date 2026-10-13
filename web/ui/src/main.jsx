import { createRoot } from "react-dom/client";
import "./index.css";
import App from "./App.jsx";
import {MseRendererProvider} from './context/MseRendererContext';

createRoot(document.getElementById("root")).render(
  <MseRendererProvider corpusUrl={import.meta.env.VITE_MSE_CORPUS_URL} defaultStyle={import.meta.env.VITE_MSE_STYLE}>
    <App />
  </MseRendererProvider>
);
