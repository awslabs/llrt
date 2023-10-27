import { useEffect, useState } from "react";
import TodoItem from "./TodoItem";
import "./TodoList.css";
import CreateTodo from "./CreateTodo";

type Props = {
  items: Todo[];
};

export type Todo = {
  id: string;
  text: string;
  createdDate: string;
  completedDate?: string;
};

const BASE_URL = `${(globalThis.window && window.location.href) || "/"}api/`;

const API = {
  deleteTodo: async (id: string) => {
    const response = await fetch(`${BASE_URL}${id}`, {
      method: "DELETE",
    });
    if (!response.ok) {
      throw new Error("Could not create todo");
    }
    await response.text();
  },

  createTodo: async (text: string) => {
    const response = await fetch(BASE_URL, {
      method: "POST",
      body: JSON.stringify({
        text,
      }),
    });
    if (!response.ok) {
      throw new Error("Could not create todo");
    }
    return await response.json();
  },

  updateTodo: async (todo: Todo) => {
    const response = await fetch(`${BASE_URL}${todo.id}`, {
      method: "PUT",
      body: JSON.stringify(todo),
    });
    if (!response.ok) {
      throw new Error("Could not update todo");
    }
    return await response.json();
  },
};

function TodoList({ items: initialItems }: Props) {
  const [items, setItems] = useState<Todo[]>(initialItems);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | undefined>();

  const handleCreate = (text: string) => {
    setError(undefined);
    setLoading(true);
    API.createTodo(text)
      .then((todo) => {
        setItems([...items, todo]);
      })
      .catch((e) => {
        setError(e.message);
      })
      .finally(() => {
        setLoading(false);
      });
  };

  const handleDelete = (id: string) => {
    let index = items.findIndex((todo) => todo.id === id);
    setLoading(true);
    API.deleteTodo(id)
      .then(() => {
        items.splice(index, 1);
        setItems([...items]);
      })
      .catch((e) => {
        setError(e.message);
      })
      .finally(() => {
        setLoading(false);
      });
  };

  const handleComplete = (id: string) => {
    setError(undefined);
    const index = items.findIndex((todo) => todo.id === id);
    const item = { ...items[index] }; //copy object

    if (!item.completedDate) {
      item.completedDate = new Date().toISOString();
    } else {
      delete item.completedDate;
    }
    setLoading(true);
    API.updateTodo(item)
      .then((todo) => {
        items[index] = todo;
        setItems([...items]);
      })
      .catch((e) => {
        setError(e.message);
      })
      .finally(() => {
        setLoading(false);
      });
  };

  return (
    <div className={`todo-list ${loading ? "loading" : ""}`}>
      {error && <p className="error-message">Error: {error}</p>}
      <ul>
        {items.map((todo) => (
          <TodoItem
            onComplete={handleComplete}
            onDelete={handleDelete}
            key={todo.id}
            item={todo}
          />
        ))}
      </ul>
      <CreateTodo onCreate={handleCreate} />
    </div>
  );
}

export default TodoList;
