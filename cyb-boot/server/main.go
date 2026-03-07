package main

import (
	"archive/zip"
	"bytes"
	"encoding/base64"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"time"
)

var (
	artifactsDir string
	rateLimiter  = &limiter{requests: make(map[string]time.Time)}
)

type limiter struct {
	mu       sync.Mutex
	requests map[string]time.Time
}

func (l *limiter) allow(ip string) bool {
	l.mu.Lock()
	defer l.mu.Unlock()
	if last, ok := l.requests[ip]; ok && time.Since(last) < 3*time.Second {
		return false
	}
	l.requests[ip] = time.Now()
	return true
}

type bootRequest struct {
	Platform string `json:"platform"`
	Data     string `json:"data"` // base64-encoded encrypted bootstrap
}

var validPlatforms = map[string]bool{
	"aarch64-apple-darwin":       true,
	"x86_64-apple-darwin":        true,
	"x86_64-pc-windows-msvc":     true,
	"x86_64-unknown-linux-musl":  true,
	"aarch64-unknown-linux-musl": true,
}

func handleBoot(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	ip := r.RemoteAddr
	if fwd := r.Header.Get("X-Forwarded-For"); fwd != "" {
		ip = strings.Split(fwd, ",")[0]
	}
	if !rateLimiter.allow(ip) {
		http.Error(w, "Rate limit exceeded", http.StatusTooManyRequests)
		return
	}

	var req bootRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "Invalid JSON", http.StatusBadRequest)
		return
	}

	if !validPlatforms[req.Platform] {
		http.Error(w, "Unknown platform: "+req.Platform, http.StatusBadRequest)
		return
	}

	// Decode boot data
	bootData, err := base64.StdEncoding.DecodeString(req.Data)
	if err != nil {
		http.Error(w, "Invalid base64 data", http.StatusBadRequest)
		return
	}

	isDarwin := strings.Contains(req.Platform, "darwin")

	if isDarwin {
		// macOS: read pre-signed notarized boot_cyb.zip and append boot.dat
		templatePath := filepath.Join(artifactsDir, "boot_cyb.zip")
		templateZip, err := os.ReadFile(templatePath)
		if err != nil {
			log.Printf("boot_cyb.zip not found: %s", templatePath)
			http.Error(w, "macOS app bundle not available", http.StatusServiceUnavailable)
			return
		}

		outBuf, err := appendToZip(templateZip, "boot.dat", bootData)
		if err != nil {
			log.Printf("Failed to append boot.dat to zip: %v", err)
			http.Error(w, "Server error", http.StatusInternalServerError)
			return
		}

		w.Header().Set("Content-Type", "application/zip")
		w.Header().Set("Content-Disposition", `attachment; filename="boot_cyb.zip"`)
		w.Header().Set("Content-Length", fmt.Sprintf("%d", outBuf.Len()))
		w.Write(outBuf.Bytes())
		log.Printf("Served boot_cyb.zip + boot.dat for %s (%d bytes)", req.Platform, outBuf.Len())
		return
	}

	// Windows/Linux: read artifact binary and build zip
	artifactPath := filepath.Join(artifactsDir, "cyb-boot-"+req.Platform)
	binary, err := os.ReadFile(artifactPath)
	if err != nil {
		log.Printf("Artifact not found: %s", artifactPath)
		http.Error(w, "Binary not available for platform", http.StatusServiceUnavailable)
		return
	}

	var buf bytes.Buffer
	zw := zip.NewWriter(&buf)

	isWindows := strings.Contains(req.Platform, "windows")
	if isWindows {
		addToZip(zw, "cyb-boot/cyb-boot.exe", binary, 0755)
		addToZip(zw, "cyb-boot/boot.dat", bootData, 0644)
	} else {
		addToZip(zw, "cyb-boot/cyb-boot", binary, 0755)
		addToZip(zw, "cyb-boot/boot.dat", bootData, 0644)
	}

	if err := zw.Close(); err != nil {
		log.Printf("Zip close error: %v", err)
		http.Error(w, "Server error", http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/zip")
	w.Header().Set("Content-Disposition", `attachment; filename="cyb-boot.zip"`)
	w.Header().Set("Content-Length", fmt.Sprintf("%d", buf.Len()))
	w.Write(buf.Bytes())

	log.Printf("Served %s (%d bytes)", req.Platform, buf.Len())
}

func addToZip(zw *zip.Writer, name string, data []byte, mode os.FileMode) {
	header := &zip.FileHeader{
		Name:   name,
		Method: zip.Deflate,
	}
	header.SetMode(mode)
	w, err := zw.CreateHeader(header)
	if err != nil {
		log.Printf("zip add error for %s: %v", name, err)
		return
	}
	w.Write(data)
}

// appendToZip copies all entries from an existing zip and appends a new file.
func appendToZip(existingZip []byte, newName string, newData []byte) (*bytes.Buffer, error) {
	reader, err := zip.NewReader(bytes.NewReader(existingZip), int64(len(existingZip)))
	if err != nil {
		return nil, fmt.Errorf("reading template zip: %w", err)
	}

	var buf bytes.Buffer
	writer := zip.NewWriter(&buf)

	// Copy all existing entries
	for _, f := range reader.File {
		fw, err := writer.CreateHeader(&f.FileHeader)
		if err != nil {
			return nil, fmt.Errorf("creating header for %s: %w", f.Name, err)
		}
		rc, err := f.Open()
		if err != nil {
			return nil, fmt.Errorf("opening %s: %w", f.Name, err)
		}
		_, err = io.Copy(fw, rc)
		rc.Close()
		if err != nil {
			return nil, fmt.Errorf("copying %s: %w", f.Name, err)
		}
	}

	// Append new file
	header := &zip.FileHeader{
		Name:   newName,
		Method: zip.Deflate,
	}
	header.SetMode(0644)
	fw, err := writer.CreateHeader(header)
	if err != nil {
		return nil, fmt.Errorf("creating %s header: %w", newName, err)
	}
	fw.Write(newData)

	if err := writer.Close(); err != nil {
		return nil, fmt.Errorf("closing zip: %w", err)
	}

	return &buf, nil
}

func main() {
	artifactsDir = os.Getenv("CYB_BOOT_ARTIFACTS")
	if artifactsDir == "" {
		artifactsDir = "artifacts"
	}

	mux := http.NewServeMux()
	mux.HandleFunc("/", handleBoot)

	addr := ":8098"
	log.Printf("cyb-boot-server listening on %s (artifacts: %s)", addr, artifactsDir)
	if err := http.ListenAndServe(addr, mux); err != nil {
		log.Fatal(err)
	}
}
