package server

import (
	"fmt"
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
	var ipStr string

	{
		rawIpStr := r.Header.Get("X-Forwarded-For")
		// The following would likely not work, as RemoteAddr likely has a port number
		/*
			if rawIpStr == "" {
				rawIpStr = r.RemoteAddr
			}
		*/
		ip := net.ParseIP(rawIpStr)
		if ip != nil {
			ipStr = ip.String()
		}
	}

	if ipStr != "" {
		h.ipMu.RLock()
		count := h.ipConns[ipStr]
		h.ipMu.RUnlock()
		if count >= 10 {
			fmt.Printf("Blocked %s for too many connections\n", ipStr)
			http.Error(w, "Too many connections", 429)
			return
		}
	}

	conn, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		// at this point, upgrader already responded with error
		fmt.Println("upgrade error", err)
		return
	}

	// The connection is official now (but don't wait for registration)
	if ipStr != "" {
		h.ipMu.Lock()
		defer h.ipMu.Unlock()
		h.ipConns[ipStr]++
	}

	h.register <- NewSocketClient(conn, ipStr)
}
