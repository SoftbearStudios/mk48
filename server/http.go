package server

import (
	"log"
	"net"
	"net/http"
)

func (h *Hub) ServeIndex(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Access-Control-Allow-Origin", "*")
	w.Header().Set("Content-Type", "application/json")
	buf, ok := h.statusJSON.Load().([]byte)
	if ok {
		_, _ = w.Write(buf)
	}
}

func (h *Hub) ServeSocket(w http.ResponseWriter, r *http.Request) {
	ipStr := r.Header.Get("X-Forwarded-For")
	if ipStr == "" {
		ipStr = r.RemoteAddr
	}
	ip := net.ParseIP(ipStr)

	if ip != nil {
		h.ipMu.RLock()
		count := h.ipConns[ipStr]
		h.ipMu.RUnlock()
		if count >= 10 {
			http.Error(w, "Too many connections", 429)
			return
		}
	}

	conn, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		// at this point, upgrader already responded with error
		log.Println("upgrade error", err)
		return
	}

	// The connection is official now (but don't wait for registration)
	if ip != nil {
		h.ipMu.Lock()
		defer h.ipMu.Unlock()
		h.ipConns[ipStr]++
	}

	h.register <- NewSocketClient(conn, ip)
}
