/*
 * Copyright (c) 2020 Softbear Studios - All Rights Reserved
 */
// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package fs

import (
	"bytes"
	"fmt"
	"github.com/aws/aws-sdk-go/aws"
	"github.com/aws/aws-sdk-go/aws/session"
	"github.com/aws/aws-sdk-go/service/s3"
	"strings"
)

type S3Filesystem struct {
	svc          *s3.S3
	staticBucket string
}

func NewS3Filesystem(session *session.Session, stage string) (*S3Filesystem, error) {
	s3Filesystem := &S3Filesystem{svc: s3.New(session)}

	s3Filesystem.staticBucket = "mk48-" + stage + "-static"

	return s3Filesystem, nil
}

var s3ContentTypes = map[string]string{
	".json": "application/json",
}

func (s3Filesystem *S3Filesystem) UploadStaticFile(filename string, secondsCache int, data []byte) error {
	readSeeker := bytes.NewReader(data)

	// Patch S3's limited vocabulary of default content types
	var contentType *string
	for ext, mime := range s3ContentTypes {
		if strings.HasSuffix(filename, ext) {
			contentType = &mime
			break
		}
	}

	req, _ := s3Filesystem.svc.PutObjectRequest(&s3.PutObjectInput{
		Bucket:       aws.String(s3Filesystem.staticBucket),
		Key:          aws.String(filename),
		Body:         readSeeker,
		CacheControl: aws.String(fmt.Sprintf("no-transform, public, max-age=%d", secondsCache)),
		ContentType:  contentType,
	})
	err := req.Send()
	return err
}
