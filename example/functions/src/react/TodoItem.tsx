import { Todo } from "./TodoList";

type Props = {
  item: Todo;
  onDelete: (id: string) => void;
  onComplete: (id: string) => void;
};

function TodoItem({
  item: { id, text, createdDate, completedDate },
  onDelete,
  onComplete,
}: Props) {
  const handleDeleteClick = (e: React.MouseEvent<HTMLElement>) => {
    e.stopPropagation();
    onDelete(id);
  };
  return (
    <li>
      <div onClick={() => onComplete(id)} className="todo-item">
        <span>{completedDate ? "✔" : "⏲"}</span>
        <span className={`todo-text ${completedDate ? "completed" : ""}`}>
          {text}
        </span>
        <span onClick={handleDeleteClick}>🗑️</span>
      </div>
    </li>
  );
}

export default TodoItem;
