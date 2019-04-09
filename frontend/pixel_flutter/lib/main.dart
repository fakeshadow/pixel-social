import 'package:flutter/material.dart';
import 'package:http/http.dart' as http;
import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter/blocs/Blocs.dart';
import 'package:pixel_flutter/models/Topic.dart';

import 'package:pixel_flutter/components/NavigationBar/NavBarCommon.dart';
import 'package:pixel_flutter/components/NavigationBar/TabNavBar.dart';

import 'package:pixel_flutter/components/Loader/CenterLoader.dart';
import 'package:pixel_flutter/components/Loader/BottomLoader.dart';

import './components//History/HistoryLimit.dart';
import './Views/ProfilePage.dart';

void main() => runApp(RootApp());

class RootApp extends StatefulWidget {
  @override
  RootAppState createState() => RootAppState();
}

class RootAppState extends State<RootApp> {
  @override
  Widget build(BuildContext context) {
    return MaterialApp(
        routes: {
          '/profile': (context) => ProfilePage(),
          '/community': (context) => RootApp(),
        },
        theme: ThemeData(
            brightness: Brightness.light,
            primarySwatch: Colors.blue,
            accentColor: Colors.deepPurple),
        navigatorObservers: [HistoryLimit(10)],
        home: CommunityPage());
  }
}

class CommunityPage extends StatefulWidget {
  @override
  _CommunityPage createState() => _CommunityPage();
}

class _CommunityPage extends State<CommunityPage> {
  final _scrollController = ScrollController();
  final TopicBloc _topicBloc = TopicBloc(httpClient: http.Client());
  final _scrollThreshold = 200.0;

  final _scaffoldKey = new GlobalKey<ScaffoldState>();

  _CommunityPage() {
    _scrollController.addListener(_onScroll);
    _topicBloc.dispatch(TopicAPI());
  }

  @override
  Widget build(BuildContext context) {
    return BlocBuilder(
        bloc: _topicBloc,
        builder: (BuildContext context, TopicState state) {
          return Scaffold(
            key: _scaffoldKey,
            bottomNavigationBar: TabNavBar(1),
            endDrawer: Container(
              child: Center(child: Text('abcdefg')),
            ),
            body: CustomScrollView(
                controller: _scrollController,
                slivers: <Widget>[
                  NavBarCommon(title: 'test', isClose: false),
                  Sliverlist(state)
                ]),
          );
        });
  }

  @override
  void dispose() {
    _topicBloc.dispose();
    super.dispose();
  }

  void _onScroll() {
    final maxScroll = _scrollController.position.maxScrollExtent;
    final currentScroll = _scrollController.position.pixels;
    if (maxScroll - currentScroll <= _scrollThreshold) {
      _topicBloc.dispatch(TopicAPI());
    }
  }
}

class Sliverlist extends StatelessWidget {
  final state;

  Sliverlist(this.state);

  @override
  Widget build(BuildContext context) {
    if (state is TopicError) {
      return CenterLoader();
    }
    if (state is TopicLoaded) {
      if (state.topics.isEmpty) {
        return CenterLoader();
      }
      return SliverList(
        delegate: SliverChildBuilderDelegate((context, index) {
          return index >= state.topics.length
              ? BottomLoader()
              : Listtile(state.topics[index]);
        },
            childCount: state.hasReachedMax
                ? state.topics.length
                : state.topics.length + 1),
      );
    } else {
      return CenterLoader();
    }
  }
}

class Listtile extends StatelessWidget {
  final String url = 'http://192.168.1.197:3200';
  final Topic topic;

  Listtile(this.topic);

  @override
  Widget build(BuildContext context) {
    return ListTile(
      leading: InkWell(
        onTap: () => print('Avatar pressed'),
        child: CircleAvatar(
          child: Container(
            decoration: BoxDecoration(
                shape: BoxShape.circle,
                image: DecorationImage(
                  fit: BoxFit.fill,
                  image: NetworkImage(
                      url + '${topic.avatarUrl}'),
                )),
          ),
          backgroundColor: Colors.white10,
        ),
      ),
      title: InkWell(
        onTap: () => print('pressed'),
        child: Text(
          '${topic.title}',
          style: TextStyle(
            fontSize: 16.0,
            fontWeight: FontWeight.w600,
          ),
        ),
      ),
      subtitle: Text(
        '${topic.id}    ${topic.username}    ${topic.lastReplyTime}',
        style:
        TextStyle(fontSize: 12.0, fontWeight: FontWeight.w600),
      ),
      trailing: Icon(IconData(0x0)),
    );
  }
}