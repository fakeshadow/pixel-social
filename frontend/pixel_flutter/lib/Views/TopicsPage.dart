import 'package:flutter/material.dart';
import '../components/NavigationBar/NavBarCommon.dart';
import '../components/NavigationBar/TabNavBar.dart';

import 'package:http/http.dart' as http;
import 'dart:async';
import 'dart:convert';

class TopicsPage extends StatefulWidget {
  @override
  _TopicsPageState createState() => _TopicsPageState();
}

class _TopicsPageState extends State<TopicsPage> {
  final String url = "http://192.168.1.197:3100/api/post/test";
  List data;

  Future<String> getPosts() async {
    var res = await http
        .get(Uri.encodeFull(url), headers: {"Accept": "application/json"});
    setState(() {
      var resBody = json.decode(res.body);
      data = resBody;
    });
    return "Success";
  }

  @override
  void initState() {
    super.initState();
    this.getPosts();
  }

  openDetail() {
    Navigator.of(context).pushNamed('/posts');
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
        bottomNavigationBar: TabNavBar(1),
        body: CustomScrollView(
          slivers: <Widget>[
            NavBarCommon(title: 'Topics', isClose: true),
            SliverList(
              delegate:
                  SliverChildBuilderDelegate((BuildContext context, int index) {
                if (data == null) {
                  return Container();
                }
                return Container(
                    child: Card(
                        elevation: 10,
                        margin: EdgeInsets.only(
                            left: 10.0, right: 10.0, top: 4, bottom: 4),
                        child: Padding(
                          padding: const EdgeInsets.all(8.0),
                          child: Row(
                            mainAxisAlignment: MainAxisAlignment.start,
                            children: <Widget>[
                              Image.asset(
                                  data[index]["AvatarUrl"] != null
                                      ? data[index]["AvatarUrl"]
                                      : 'assets/test2.png',
                                  width: 40,
                                  fit: BoxFit.contain),
                              SizedBox(
                                width: 10,
                              ),
                              Column(
                                crossAxisAlignment: CrossAxisAlignment.start,
                                children: <Widget>[
                                  Row(
                                    children: <Widget>[
                                      Text(data[index]["uid"].toString()),
                                      SizedBox(
                                        width: 10,
                                      ),
                                      Text(data[index]["createdAt"].toString()),
                                    ],
                                  ),
                                  // insert row for tags here
                                  SizedBox(
                                    width: 10,
                                  ),
                                  SizedBox(height: 5),
                                  Text(data[index]["postData"]),
                                  SizedBox(height: 5),
                                  InkWell(
                                      child: Text('Press for detail'),
                                      onTap: openDetail)
                                ],
                              )
                            ],
                          ),
                        )));
              }, childCount: data == null ? 0 : data.length),
            ),
          ],
        ));
  }
}
