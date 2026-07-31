import { useState } from "react";

type Page = "settings" | "history" | "ocr" | "ai";

function App() {
  const [currentPage, setCurrentPage] = useState<Page>("settings");

  return (
    <div className="flex h-screen bg-gray-50 dark:bg-gray-900">
      <nav className="w-48 border-r border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 p-4">
        <h1 className="text-lg font-bold text-primary mb-6">FalconShot</h1>
        <ul className="space-y-2">
          {(["settings", "history", "ocr", "ai"] as Page[]).map((page) => (
            <li key={page}>
              <button
                onClick={() => setCurrentPage(page)}
                className={`w-full text-left px-3 py-2 rounded-md text-sm ${
                  currentPage === page
                    ? "bg-primary/10 text-primary font-medium"
                    : "text-gray-600 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700"
                }`}
              >
                {page === "settings" && "设置"}
                {page === "history" && "历史记录"}
                {page === "ocr" && "OCR 结果"}
                {page === "ai" && "AI 分析"}
              </button>
            </li>
          ))}
        </ul>
      </nav>
      <main className="flex-1 p-6 overflow-auto">
        {currentPage === "settings" && <SettingsPage />}
        {currentPage === "history" && <HistoryPage />}
        {currentPage === "ocr" && <OcrPage />}
        {currentPage === "ai" && <AiPage />}
      </main>
    </div>
  );
}

function SettingsPage() {
  return (
    <div>
      <h2 className="text-xl font-semibold mb-4 text-gray-800 dark:text-gray-100">设置</h2>
      <p className="text-gray-500 dark:text-gray-400">设置页面开发中...</p>
    </div>
  );
}

function HistoryPage() {
  return (
    <div>
      <h2 className="text-xl font-semibold mb-4 text-gray-800 dark:text-gray-100">历史记录</h2>
      <p className="text-gray-500 dark:text-gray-400">历史记录页面开发中...</p>
    </div>
  );
}

function OcrPage() {
  return (
    <div>
      <h2 className="text-xl font-semibold mb-4 text-gray-800 dark:text-gray-100">OCR 结果</h2>
      <p className="text-gray-500 dark:text-gray-400">OCR 结果面板开发中...</p>
    </div>
  );
}

function AiPage() {
  return (
    <div>
      <h2 className="text-xl font-semibold mb-4 text-gray-800 dark:text-gray-100">AI 分析</h2>
      <p className="text-gray-500 dark:text-gray-400">AI 分析面板开发中...</p>
    </div>
  );
}

export default App;
