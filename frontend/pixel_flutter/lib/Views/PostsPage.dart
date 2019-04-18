import 'package:flutter/material.dart';
import '../components/NavigationBar/SliverNavBar.dart';
import '../components/NavigationBar/TabNavBar.dart';

import 'package:http/http.dart' as http;
import 'dart:async';
import 'dart:convert';

class PostsPage extends StatefulWidget {
  final String _title;
  PostsPage(this._title);

  @override
  _PostsPageState createState() => _PostsPageState();
}

class _PostsPageState extends State<PostsPage> {
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

  _showPosts() {
    Navigator.of(context).pushNamed('PostsPage');
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
        bottomNavigationBar: TabNavBar(1),
        body: CustomScrollView(
          slivers: <Widget>[
            SliverNavBar(title: widget._title),
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
                              Image.asset('assets/test2.png',
                                  width: 40, fit: BoxFit.contain),
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
                                      onTap: _showPosts)
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
