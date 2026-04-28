import { useState } from "react";
import TabSshForward from "./components/TabSshForward";
import TabK8sForward from "./components/TabK8sForward";
import TabHttpProxy from "./components/TabHttpProxy";

const TABS = ["SSH端口转发", "K8s服务转发", "HTTP代理"];

export default function App() {
  const [activeTab, setActiveTab] = useState(0);

  return (
    <div className="k8s-forward">
      <div className="tabs">
        {TABS.map((t, i) => (
          <button key={t} className={`tab ${activeTab === i ? "active" : ""}`} onClick={() => setActiveTab(i)}>
            {t}
          </button>
        ))}
      </div>
      {activeTab === 0 && <TabSshForward />}
      {activeTab === 1 && <TabK8sForward />}
      {activeTab === 2 && <TabHttpProxy />}
    </div>
  );
}
