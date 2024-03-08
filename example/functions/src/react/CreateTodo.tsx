type Props = {
  onCreate: (text: string) => void;
};

function CreateTodo({ onCreate }: Props) {
  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") {
      const target = e.target as any;
      onCreate(target.value);
      target.value = "";
    }
  };

  return (
    <input
      onKeyDown={handleKeyDown}
      className="create-todo"
      type="text"
      placeholder="What do you want to do?"
    />
  );
}

export default CreateTodo;
