package server

import (
	"log"
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
	conn, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		log.Println("upgrade error", err)
		return
	}

	h.register <- NewSocketClient(conn)
}
