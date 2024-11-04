import { useState } from "react";
import "./App.css";

function App() {
  const [count, setCount] = useState(0);

  return (
    <>
      <div className="card" style={{ fontSize: 20 }}>
        <div className="flex flex-row">
          <h1>hi</h1>
          fast hiiii
        </div>
        <button onClick={() => setCount((count) => count + 1)}>
          Increment
        </button>
        <p>{count}</p>
      </div>
    </>
  );
}

export default App;
