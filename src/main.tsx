import React from "react"
import ReactDOM from "react-dom/client"
import { MotionConfig } from "motion/react"
import App from "./App"
import "@fontsource-variable/bricolage-grotesque"
import "@fontsource-variable/hanken-grotesk"
import "@fontsource-variable/geist-mono"
import "./index.css"

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <MotionConfig reducedMotion="user">
      <App />
    </MotionConfig>
  </React.StrictMode>,
)
