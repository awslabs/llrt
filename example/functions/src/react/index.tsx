import ReactDOM from "react-dom";

import App from "./App";

ReactDOM.hydrate(
  <App todoItems={(window && window.todoItems) || []} />,
  document.getElementById("root")
);
