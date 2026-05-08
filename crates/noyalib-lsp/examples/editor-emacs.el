;;; SPDX-License-Identifier: Apache-2.0 OR MIT
;;;
;;; noyalib-lsp configuration for Emacs (≥ 29) via the built-in
;;; `eglot' package. Tested on emacs-mac, emacs-plus, and the
;;; vanilla GNU Emacs 29 release line. eglot is bundled — no
;;; external package install required.
;;;
;;; Drop this snippet into your Emacs init file
;;; (`~/.emacs.d/init.el' or `~/.config/emacs/init.el').

(require 'eglot)

;; 1. Register noyalib-lsp as the language server for yaml-mode
;;    and yaml-ts-mode (Emacs 29's tree-sitter mode).
(with-eval-after-load 'eglot
  (add-to-list 'eglot-server-programs
               '((yaml-mode yaml-ts-mode) . ("noyalib-lsp"))))

;; 2. Auto-start eglot on YAML buffers.
(add-hook 'yaml-mode-hook #'eglot-ensure)
(add-hook 'yaml-ts-mode-hook #'eglot-ensure)

;; 3. Format-on-save via the LSP textDocument/formatting handler.
(defun noyalib-lsp/format-on-save ()
  "Run `eglot-format-buffer' on save in YAML buffers managed by eglot."
  (when (and (eglot-managed-p)
             (member major-mode '(yaml-mode yaml-ts-mode)))
    (eglot-format-buffer)))

(add-hook 'before-save-hook #'noyalib-lsp/format-on-save)

;; 4. Optional keymaps. Adjust to your scheme.
(with-eval-after-load 'eglot
  (define-key eglot-mode-map (kbd "C-c h") #'eldoc)            ; hover at point
  (define-key eglot-mode-map (kbd "C-c f") #'eglot-format-buffer)
  (define-key eglot-mode-map (kbd "C-c r") #'eglot-rename))

;; Verifying the install:
;;
;;   M-x package-install RET yaml-mode RET    ; if you don't already have it
;;   M-: (executable-find "noyalib-lsp")      ; should return absolute path
;;   M-x find-file RET test.yaml RET          ; eglot announces "[eglot]" in modeline
;;   C-c h                                    ; hover at point
;;   C-c f                                    ; explicit format-buffer
