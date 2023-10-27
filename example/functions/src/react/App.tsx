import logo from "./logo.svg";
import "./App.css";
import TodoList, { Todo } from "./TodoList";

type Props = {
  todoItems?: Todo[];
};

function App({ todoItems = [] }: Props) {
  return (
    <div className="app">
      <header className="header">
        <img src={logo} className="logo" alt="logo" />
      </header>
      <main className="main">
        <h1>LLRT React TODO</h1>
        <TodoList items={todoItems} />
      </main>
    </div>
  );
}

export default App;
