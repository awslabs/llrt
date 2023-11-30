import logo from "./logo.svg";
import "./App.css";
import TodoList, { Todo } from "./TodoList";

type Props = {
  todoItems?: Todo[];
  releaseName?: string;
};

function App({ todoItems = [], releaseName = "" }: Props) {
  return (
    <div className="app">
      <header className="header">
        <img src={logo} className="logo" alt="logo" />
      </header>
      <main className="main">
        <h1>LLRT React TODO - {releaseName}</h1>
        <TodoList items={todoItems} />
      </main>
    </div>
  );
}

export default App;
