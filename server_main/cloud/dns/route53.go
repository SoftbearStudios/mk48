/*
 * Copyright (c) 2020 Softbear Studios - All Rights Reserved
 */
// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package dns

import (
	"fmt"
	"github.com/aws/aws-sdk-go/aws"
	"github.com/aws/aws-sdk-go/aws/session"
	"github.com/aws/aws-sdk-go/service/route53"
	"net"
)

type Route53DNS struct {
	svc    *route53.Route53
	domain string
	zoneID string
}

func NewRoute53DNS(session *session.Session, domain string, zoneID string) (*Route53DNS, error) {
	route53DNS := &Route53DNS{svc: route53.New(session)}

	route53DNS.domain = domain
	route53DNS.zoneID = zoneID

	return route53DNS, nil
}

func (route53DNS *Route53DNS) UpdateRoute(region string, slot int, address net.IP) error {
	request := &route53.ChangeResourceRecordSetsInput{
		ChangeBatch: &route53.ChangeBatch{
			Changes: []*route53.Change{
				{
					Action: aws.String("UPSERT"),
					ResourceRecordSet: &route53.ResourceRecordSet{
						Name: aws.String(fmt.Sprintf("ws-%s-%d.%s", region, slot, route53DNS.domain)),
						Type: aws.String("A"),
						ResourceRecords: []*route53.ResourceRecord{
							{
								Value: aws.String(address.String()),
							},
						},
						TTL: aws.Int64(60),
					},
				},
			},
		},
		HostedZoneId: aws.String(route53DNS.zoneID),
	}
	_, err := route53DNS.svc.ChangeResourceRecordSets(request)
	return err
}
