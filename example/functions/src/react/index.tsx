import ReactDOM from "react-dom";

import App from "./App";

ReactDOM.hydrate(
  <App
    todoItems={(window && window.todoItems) || []}
    releaseName={window.releaseName}
  />,
  document.getElementById("root")
);
