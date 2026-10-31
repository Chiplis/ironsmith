import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import LoadingSmith from '../src/components/layout/LoadingSmith';
function Harness() {
  const [input, setInput] = useState({ progress: 0, ready: false });
  window.setSmithInput = setInput;
  return <LoadingSmith {...input} onComplete={() => { window.smithComplete = true; }} />;
}
createRoot(document.getElementById('root')).render(<Harness />);
