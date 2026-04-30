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
      <div className="tab-content">
        <div style={{ display: activeTab === 0 ? "block" : "none" }}><TabSshForward /></div>
        <div style={{ display: activeTab === 1 ? "block" : "none" }}><TabK8sForward /></div>
        <div style={{ display: activeTab === 2 ? "block" : "none" }}><TabHttpProxy /></div>
      </div>
    </div>
  );
}
