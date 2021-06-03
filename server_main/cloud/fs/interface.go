// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package fs

type Filesystem interface {
	UploadStaticFile(filename string, secondsCache int, data []byte) error
}
