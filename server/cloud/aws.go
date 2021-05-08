// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package cloud

import (
	"bytes"
	"errors"
	"fmt"
	"github.com/aws/aws-sdk-go/aws"
	"github.com/aws/aws-sdk-go/aws/credentials"
	"github.com/aws/aws-sdk-go/aws/credentials/ec2rolecreds"
	"github.com/aws/aws-sdk-go/aws/ec2metadata"
	"github.com/aws/aws-sdk-go/aws/session"
	"io"
	"net"
	"net/http"
	"os"
	"os/user"
	"strconv"
	"strings"
	"time"
)

const AWSProfile = "mk48"

type UserData struct {
	Domain        string
	Region        string
	Stage         string
	ServerSlots   int
	Route53ZoneID string
}

func getAWSSession(region string) (*session.Session, error) {
	usr, osErr := user.Current()
	if osErr != nil {
		return nil, osErr
	}
	path := fmt.Sprintf("%s/.aws/credentials", usr.HomeDir)
	var creds *credentials.Credentials
	if _, statErr := os.Stat(path); statErr == nil {
		creds = credentials.NewSharedCredentials(path, AWSProfile)
	} else {
		creds = credentials.NewCredentials(&ec2rolecreds.EC2RoleProvider{Client: ec2metadata.New(session.New(aws.NewConfig()))})
	}
	sess, sessErr := session.NewSession(&aws.Config{
		Region:      aws.String(region),
		Credentials: creds,
	})
	if sessErr != nil {
		return nil, sessErr
	}
	return sess, nil
}

func getPublicIP() (net.IP, error) {
	resp, httpErr := http.Get("http://checkip.amazonaws.com")
	if httpErr != nil {
		return nil, httpErr
	}
	defer resp.Body.Close()
	body, readErr := io.ReadAll(resp.Body)
	if readErr != nil {
		return nil, readErr
	}
	ipString := strings.TrimSuffix(string(body), "\n")
	ip := net.ParseIP(ipString)
	if ip == nil {
		return nil, errors.New("Could not parse IP address '" + ipString + "'")
	}
	return ip, nil
}

func loadUserData() (data *UserData, err error) {
	client := http.Client{Timeout: time.Second / 2}
	response, err := client.Get("http://169.254.169.254/latest/user-data/")
	if err != nil {
		return
	}
	defer response.Body.Close()

	var buf bytes.Buffer
	buf.ReadFrom(response.Body)
	userData := buf.String()

	variables := strings.Split(userData, "\n")

	// Defaults
	data = &UserData{}

	for _, variable := range variables {
		equalsIndex := strings.IndexRune(variable, '=')
		if equalsIndex == -1 {
			continue
		}
		name := strings.Trim(variable[:equalsIndex], " ")
		value := strings.Trim(variable[equalsIndex+1:], "\" ")

		switch name {
		case "DOMAIN":
			data.Domain = value
		case "REGION":
			data.Region = value
		case "STAGE":
			data.Stage = value
		case "SERVER_SLOTS":
			data.ServerSlots, err = strconv.Atoi(value)
			if err != nil {
				return
			}
		case "ROUTE53_ZONEID":
			data.Route53ZoneID = value
		}
	}

	if data.Domain == "" {
		return nil, errors.New("missing domain")
	}
	if data.Region == "" {
		return nil, errors.New("missing region")
	}
	if data.Stage == "" {
		return nil, errors.New("missing stage")
	}
	if data.ServerSlots < 1 {
		return nil, errors.New("missing server slots")
	}
	if data.Route53ZoneID == "" {
		return nil, errors.New("missing route53 zoneID")
	}
	return data, nil
}
